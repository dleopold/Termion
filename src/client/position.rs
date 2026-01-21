//! Position-specific client for acquisition and statistics services.

use super::{
    ChannelState, ClientError, DutyTimeSnapshot, Position, ReadLengthHistogram, RunState,
    StatsSnapshot, YieldDataPoint,
};
use crate::proto::minknow_api::acquisition::{
    acquisition_service_client::AcquisitionServiceClient, CurrentStatusRequest,
    GetAcquisitionRunInfoRequest, MinknowStatus, StopRequest,
};
use crate::proto::minknow_api::data::{
    data_service_client::DataServiceClient, GetChannelStatesRequest,
};
use crate::proto::minknow_api::device::{
    device_service_client::DeviceServiceClient, GetChannelsLayoutRequest,
};
use crate::proto::minknow_api::protocol::{
    protocol_service_client::ProtocolServiceClient, GetCurrentProtocolRunRequest,
    PauseProtocolRequest, ProtocolPhase, ResumeProtocolRequest, StopProtocolRequest,
};
use crate::proto::minknow_api::statistics::{
    statistics_service_client::StatisticsServiceClient, stream_boxplot_request, DataSelection,
    ReadLengthType, StreamAcquisitionOutputRequest, StreamBoxplotRequest, StreamDutyTimeRequest,
    StreamReadLengthHistogramRequest,
};
use std::sync::Arc;
use std::time::Duration;
use tonic::service::Interceptor;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

#[derive(Clone)]
struct AuthInterceptor {
    token: Option<Arc<str>>,
}

impl Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(ref token) = self.token {
            let value = token
                .parse()
                .map_err(|_| tonic::Status::internal("Invalid auth token format"))?;
            request.metadata_mut().insert("local-auth", value);
        }
        Ok(request)
    }
}

type InterceptedChannel = tonic::service::interceptor::InterceptedService<Channel, AuthInterceptor>;

pub struct PositionClient {
    pub position: Position,
    acquisition: AcquisitionServiceClient<InterceptedChannel>,
    statistics: StatisticsServiceClient<InterceptedChannel>,
    data: DataServiceClient<InterceptedChannel>,
    device: DeviceServiceClient<InterceptedChannel>,
    protocol: ProtocolServiceClient<InterceptedChannel>,
}

impl PositionClient {
    pub async fn connect(
        position: Position,
        host: &str,
        auth_token: Option<Arc<str>>,
    ) -> Result<Self, ClientError> {
        Self::connect_with_timeouts(
            position,
            host,
            auth_token,
            super::DEFAULT_CONNECT_TIMEOUT,
            super::DEFAULT_REQUEST_TIMEOUT,
        )
        .await
    }

    pub async fn connect_with_timeouts(
        position: Position,
        host: &str,
        auth_token: Option<Arc<str>>,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, ClientError> {
        if position.grpc_port == 0 {
            return Err(ClientError::Connection {
                endpoint: format!("{}:{}", host, position.grpc_port),
                source: "Position has no gRPC port (not running?)".into(),
            });
        }

        let endpoint = format!("https://{}:{}", host, position.grpc_port);
        tracing::info!(
            endpoint = %endpoint,
            position = %position.name,
            "Connecting to position services"
        );

        let tls_domain = super::tls_domain_for_host(&endpoint, host)?;
        let ca_cert = super::load_ca_cert(&endpoint).await?;

        let tls_config = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(&ca_cert))
            .domain_name(tls_domain);

        let channel = Channel::from_shared(endpoint.clone())
            .map_err(|e| ClientError::Connection {
                endpoint: endpoint.clone(),
                source: Box::new(e),
            })?
            .tls_config(tls_config)
            .map_err(|e| ClientError::Connection {
                endpoint: endpoint.clone(),
                source: Box::new(e),
            })?
            .connect_timeout(connect_timeout)
            .timeout(request_timeout)
            .connect()
            .await
            .map_err(|e| ClientError::Connection {
                endpoint: endpoint.clone(),
                source: Box::new(e),
            })?;

        let interceptor = AuthInterceptor { token: auth_token };
        let acquisition =
            AcquisitionServiceClient::with_interceptor(channel.clone(), interceptor.clone());
        let statistics =
            StatisticsServiceClient::with_interceptor(channel.clone(), interceptor.clone());
        let data = DataServiceClient::with_interceptor(channel.clone(), interceptor.clone());
        let device = DeviceServiceClient::with_interceptor(channel.clone(), interceptor.clone());
        let protocol = ProtocolServiceClient::with_interceptor(channel, interceptor);

        tracing::info!(position = %position.name, "Connected to position services");
        Ok(Self {
            position,
            acquisition,
            statistics,
            data,
            device,
            protocol,
        })
    }

    pub async fn get_run_state(&mut self) -> Result<RunState, ClientError> {
        let response = self
            .acquisition
            .current_status(CurrentStatusRequest {})
            .await
            .map_err(|status| ClientError::Grpc {
                method: "current_status".into(),
                status,
            })?
            .into_inner();

        let acq_state = match MinknowStatus::try_from(response.status) {
            Ok(MinknowStatus::Ready) => RunState::Idle,
            Ok(MinknowStatus::Starting) => RunState::Starting,
            Ok(MinknowStatus::Processing) => RunState::Running,
            Ok(MinknowStatus::Finishing) => RunState::Finishing,
            Ok(MinknowStatus::ErrorStatus) => RunState::Error("Unknown error".to_string()),
            Err(_) => RunState::Idle,
        };

        // When processing, check protocol phase for more detail (mux scan, paused, etc.)
        if matches!(acq_state, RunState::Running) {
            if let Some(phase_state) = self.get_protocol_phase_state().await {
                return Ok(phase_state);
            }
        }

        Ok(acq_state)
    }

    async fn get_protocol_phase_state(&mut self) -> Option<RunState> {
        let response = self
            .protocol
            .get_current_protocol_run(GetCurrentProtocolRunRequest {})
            .await
            .ok()?
            .into_inner();

        let phase = ProtocolPhase::try_from(response.phase).ok()?;

        match phase {
            ProtocolPhase::PhasePreparingForMuxScan | ProtocolPhase::PhaseMuxScan => {
                Some(RunState::MuxScanning)
            }
            ProtocolPhase::PhasePaused
            | ProtocolPhase::PhasePausing
            | ProtocolPhase::PhaseBadTemperatureAutomaticPause
            | ProtocolPhase::PhaseFlowcellDisconnectAutomaticPause
            | ProtocolPhase::PhaseFlowcellMismatchAutomaticPause
            | ProtocolPhase::PhaseDeviceErrorAutomaticPause
            | ProtocolPhase::PhaseLowDiskSpaceAutomaticPause => Some(RunState::Paused),
            ProtocolPhase::PhaseResuming | ProtocolPhase::PhaseInitialising => {
                Some(RunState::Starting)
            }
            ProtocolPhase::PhaseSequencing => Some(RunState::Running),
            ProtocolPhase::PhaseCompleted => Some(RunState::Finishing),
            ProtocolPhase::PhaseUnknown => None,
        }
    }

    pub async fn get_acquisition_info(&mut self) -> Result<AcquisitionInfo, ClientError> {
        let state = self.get_run_state().await?;

        let response = self
            .acquisition
            .get_acquisition_info(GetAcquisitionRunInfoRequest::default())
            .await
            .map_err(|status| ClientError::Grpc {
                method: "get_acquisition_info".into(),
                status,
            })?
            .into_inner();

        let yield_summary = response.yield_summary.as_ref();

        Ok(AcquisitionInfo {
            run_id: response.run_id,
            state,
            reads_processed: yield_summary.map(|y| y.read_count as u64).unwrap_or(0),
            reads_passed: yield_summary
                .map(|y| y.basecalled_pass_read_count as u64)
                .unwrap_or(0),
            reads_failed: yield_summary
                .map(|y| y.basecalled_fail_read_count as u64)
                .unwrap_or(0),
            bases_passed: yield_summary
                .map(|y| y.basecalled_pass_bases as u64)
                .unwrap_or(0),
            bases_failed: yield_summary
                .map(|y| y.basecalled_fail_bases as u64)
                .unwrap_or(0),
        })
    }

    pub async fn get_stats(&mut self) -> Result<StatsSnapshot, ClientError> {
        let info = self.get_acquisition_info().await?;

        let total_bases = info.bases_passed + info.bases_failed;
        let total_reads = info.reads_passed + info.reads_failed;

        let mean_read_length = if total_reads > 0 {
            total_bases as f64 / total_reads as f64
        } else {
            0.0
        };

        Ok(StatsSnapshot {
            timestamp: None,
            reads_processed: info.reads_processed,
            bases_called: total_bases,
            throughput_bps: 0.0,
            throughput_gbph: 0.0,
            reads_passed: info.reads_passed,
            reads_failed: info.reads_failed,
            bases_passed: info.bases_passed,
            bases_failed: info.bases_failed,
            mean_quality: 0.0,
            mean_read_length,
            active_pores: 0,
        })
    }

    /// Stops the current acquisition.
    pub async fn stop(&mut self) -> Result<(), ClientError> {
        self.acquisition
            .stop(StopRequest::default())
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stop".into(),
                status,
            })?;
        Ok(())
    }

    /// Pauses the current protocol.
    ///
    /// This will only succeed if the protocol supports pausing (can_pause = true).
    pub async fn pause(&mut self) -> Result<(), ClientError> {
        self.protocol
            .pause_protocol(PauseProtocolRequest::default())
            .await
            .map_err(|status| ClientError::Grpc {
                method: "pause_protocol".into(),
                status,
            })?;
        Ok(())
    }

    /// Resumes a paused protocol.
    ///
    /// This will only succeed if the protocol is currently paused.
    pub async fn resume(&mut self) -> Result<(), ClientError> {
        self.protocol
            .resume_protocol(ResumeProtocolRequest::default())
            .await
            .map_err(|status| ClientError::Grpc {
                method: "resume_protocol".into(),
                status,
            })?;
        Ok(())
    }

    /// Stops the current protocol (not just acquisition).
    ///
    /// This stops the entire protocol run, not just the acquisition.
    pub async fn stop_protocol(&mut self) -> Result<(), ClientError> {
        self.protocol
            .stop_protocol(StopProtocolRequest::default())
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stop_protocol".into(),
                status,
            })?;
        Ok(())
    }

    pub async fn get_current_run_id(&mut self) -> Result<Option<String>, ClientError> {
        match self
            .acquisition
            .get_acquisition_info(GetAcquisitionRunInfoRequest::default())
            .await
        {
            Ok(response) => {
                let info = response.into_inner();
                if info.run_id.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(info.run_id))
                }
            }
            Err(status) if status.code() == tonic::Code::FailedPrecondition => Ok(None),
            Err(status) => Err(ClientError::Grpc {
                method: "get_acquisition_info".into(),
                status,
            }),
        }
    }

    pub async fn stream_acquisition_output(
        &mut self,
        run_id: &str,
    ) -> Result<impl futures::Stream<Item = Result<StatsSnapshot, ClientError>>, ClientError> {
        use futures::StreamExt;

        let request = StreamAcquisitionOutputRequest {
            acquisition_run_id: run_id.to_string(),
            ..Default::default()
        };

        let stream = self
            .statistics
            .stream_acquisition_output(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stream_acquisition_output".into(),
                status,
            })?
            .into_inner();

        Ok(stream.map(|result| {
            result
                .map(|response| {
                    let mut snapshot = StatsSnapshot::default();

                    for filtered in &response.snapshots {
                        if let Some(last) = filtered.snapshots.last() {
                            if let Some(yield_summary) = &last.yield_summary {
                                snapshot.reads_processed = yield_summary.read_count as u64;
                                snapshot.reads_passed =
                                    yield_summary.basecalled_pass_read_count as u64;
                                snapshot.reads_failed =
                                    yield_summary.basecalled_fail_read_count as u64;
                                snapshot.bases_passed = yield_summary.basecalled_pass_bases as u64;
                                snapshot.bases_failed = yield_summary.basecalled_fail_bases as u64;
                                snapshot.bases_called =
                                    snapshot.bases_passed + snapshot.bases_failed;
                            }
                        }
                    }

                    snapshot
                })
                .map_err(|status| ClientError::Grpc {
                    method: "stream_acquisition_output".into(),
                    status,
                })
        }))
    }

    pub async fn get_yield_history(
        &mut self,
        run_id: &str,
    ) -> Result<Vec<YieldDataPoint>, ClientError> {
        use futures::StreamExt;

        let request = StreamAcquisitionOutputRequest {
            acquisition_run_id: run_id.to_string(),
            ..Default::default()
        };

        let mut stream = self
            .statistics
            .stream_acquisition_output(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stream_acquisition_output".into(),
                status,
            })?
            .into_inner();

        let mut points = Vec::new();

        if let Some(Ok(response)) = stream.next().await {
            for filtered in &response.snapshots {
                for snapshot in &filtered.snapshots {
                    if let Some(yield_summary) = &snapshot.yield_summary {
                        let reads_passed = yield_summary.basecalled_pass_read_count as u64;
                        let reads_failed = yield_summary.basecalled_fail_read_count as u64;
                        let bases_passed = yield_summary.basecalled_pass_bases as u64;
                        let bases_failed = yield_summary.basecalled_fail_bases as u64;

                        points.push(YieldDataPoint {
                            seconds: snapshot.seconds,
                            reads: reads_passed + reads_failed,
                            bases: bases_passed + bases_failed,
                            reads_passed,
                            reads_failed,
                            bases_passed,
                            bases_failed,
                        });
                    }
                }
            }
        }

        points.sort_by_key(|p| p.seconds);
        points.dedup_by_key(|p| p.seconds);

        Ok(points)
    }

    pub async fn stream_duty_time(
        &mut self,
        run_id: &str,
    ) -> Result<impl futures::Stream<Item = Result<DutyTimeSnapshot, ClientError>>, ClientError>
    {
        use futures::StreamExt;
        use std::collections::HashMap;

        let request = StreamDutyTimeRequest {
            acquisition_run_id: run_id.to_string(),
            ..Default::default()
        };

        let stream = self
            .statistics
            .stream_duty_time(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stream_duty_time".into(),
                status,
            })?
            .into_inner();

        Ok(stream.map(|result| {
            result
                .map(|response| {
                    tracing::warn!(
                        pore_occupancy_len = response.pore_occupancy.len(),
                        channel_states_count = response.channel_states.len(),
                        bucket_ranges_count = response.bucket_ranges.len(),
                        "Raw duty time response"
                    );

                    if !response.pore_occupancy.is_empty() {
                        let min = response.pore_occupancy.iter().cloned().fold(f32::INFINITY, f32::min);
                        let max = response.pore_occupancy.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        let sum: f32 = response.pore_occupancy.iter().sum();
                        let avg = sum / response.pore_occupancy.len() as f32;
                        tracing::warn!(min = %min, max = %max, avg = %format!("{:.3}", avg), "Occupancy stats");
                    } else {
                        tracing::warn!("pore_occupancy is EMPTY");
                    }

                    let mut snapshot = DutyTimeSnapshot::default();

                    if let Some(range) = response.bucket_ranges.last() {
                        snapshot.time_range = (range.start, range.end);
                    }

                    let mut state_times: HashMap<ChannelState, u64> = HashMap::new();
                    for (name, data) in &response.channel_states {
                        let state = match name.as_str() {
                            "strand" | "sequencing" => ChannelState::Strand,
                            "pore" | "single_pore" => ChannelState::Pore,
                            "adapter" => ChannelState::Adapter,
                            "unavailable" | "inactive" | "saturated" | "zero" | "multiple" => {
                                ChannelState::Unavailable
                            }
                            "unblock" | "unblocking" => ChannelState::Unblock,
                            _ => ChannelState::Other,
                        };
                        let total: u64 = data.state_times.iter().sum();
                        *state_times.entry(state).or_insert(0) += total;
                    }
                    snapshot.state_times = state_times;
                    snapshot.pore_occupancy = response.pore_occupancy;

                    snapshot
                })
                .map_err(|status| ClientError::Grpc {
                    method: "stream_duty_time".into(),
                    status,
                })
        }))
    }

    pub async fn stream_read_length_histogram(
        &mut self,
        run_id: &str,
        exclude_outliers: bool,
        range: Option<(u64, u64)>,
    ) -> Result<impl futures::Stream<Item = Result<ReadLengthHistogram, ClientError>>, ClientError>
    {
        use futures::StreamExt;

        let outlier_percent = if exclude_outliers { 0.01 } else { 0.0 };

        let data_selection = range.map(|(min, max)| DataSelection {
            start: min as i64,
            end: max as i64,
            step: 0,
        });

        let request = StreamReadLengthHistogramRequest {
            acquisition_run_id: run_id.to_string(),
            read_length_type: ReadLengthType::EstimatedBases as i32,
            discard_outlier_percent: outlier_percent,
            poll_time_seconds: 30,
            data_selection,
            ..Default::default()
        };

        let stream = self
            .statistics
            .stream_read_length_histogram(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stream_read_length_histogram".into(),
                status,
            })?
            .into_inner();

        Ok(stream.map(move |result| {
            result
                .map(|response| {
                    let bucket_ranges: Vec<(u64, u64)> = response
                        .bucket_ranges
                        .iter()
                        .map(|r| (r.start, r.end))
                        .collect();

                    let (bucket_values, n50) = response
                        .histogram_data
                        .first()
                        .map(|data| (data.bucket_values.clone(), data.n50))
                        .unwrap_or_default();

                    ReadLengthHistogram {
                        bucket_ranges,
                        bucket_values,
                        n50,
                        outliers_excluded: exclude_outliers,
                        outlier_percent,
                        requested_range: range,
                        source_data_end: response.source_data_end,
                    }
                })
                .map_err(|status| ClientError::Grpc {
                    method: "stream_read_length_histogram".into(),
                    status,
                })
        }))
    }

    pub async fn get_mean_quality(&mut self, run_id: &str) -> Result<Option<f32>, ClientError> {
        use futures::StreamExt;

        let request = StreamBoxplotRequest {
            acquisition_run_id: run_id.to_string(),
            data_type: stream_boxplot_request::BoxplotType::Qscore as i32,
            dataset_width: 10,
            poll_time: 60,
        };

        let mut stream = self
            .statistics
            .stream_basecall_boxplots(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "stream_basecall_boxplots".into(),
                status,
            })?
            .into_inner();

        if let Some(Ok(response)) = stream.next().await {
            if let Some(last_dataset) = response.datasets.last() {
                return Ok(Some(last_dataset.q50));
            }
        }

        Ok(None)
    }

    pub async fn get_channel_states(
        &mut self,
        channel_count: u32,
    ) -> Result<super::ChannelStatesSnapshot, ClientError> {
        use futures::StreamExt;
        use std::collections::HashMap;

        let request = GetChannelStatesRequest {
            first_channel: 1,
            last_channel: channel_count,
            use_channel_states_ids: Some(false),
            wait_for_processing: true,
            ..Default::default()
        };

        let mut stream = self
            .data
            .get_channel_states(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "get_channel_states".into(),
                status,
            })?
            .into_inner();

        let mut state_counts: HashMap<String, usize> = HashMap::new();
        let mut states: Vec<String> = vec![String::new(); channel_count as usize];

        // Take only the first response - this is a streaming RPC that runs indefinitely
        if let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    for channel_data in response.channel_states {
                        let state_name = match channel_data.state {
                            Some(crate::proto::minknow_api::data::get_channel_states_response::channel_state_data::State::StateName(name)) => name,
                            Some(crate::proto::minknow_api::data::get_channel_states_response::channel_state_data::State::StateId(id)) => format!("state_{}", id),
                            None => "unknown".to_string(),
                        };

                        let channel_idx = (channel_data.channel as usize).saturating_sub(1);
                        if channel_idx < states.len() {
                            states[channel_idx] = state_name.clone();
                        }
                        *state_counts.entry(state_name).or_insert(0) += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Error in channel states stream");
                }
            }
        }

        tracing::info!(
            channel_count = channel_count,
            states_received = state_counts.values().sum::<usize>(),
            unique_states = state_counts.len(),
            "Got channel states"
        );

        for (state, count) in &state_counts {
            tracing::debug!(state = %state, count = count, "Channel state count");
        }

        Ok(super::ChannelStatesSnapshot {
            channel_count: channel_count as usize,
            states,
            state_counts,
        })
    }

    pub async fn get_channel_layout(&mut self) -> Result<super::ChannelLayout, ClientError> {
        use std::collections::BTreeSet;

        let response = self
            .device
            .get_channels_layout(GetChannelsLayoutRequest {})
            .await
            .map_err(|status| ClientError::Grpc {
                method: "get_channels_layout".into(),
                status,
            })?
            .into_inner();

        let channel_count = response.channel_records.len();
        let mut raw_coords: Vec<(u32, u32)> = vec![(0, 0); channel_count];

        let mut unique_x: BTreeSet<u32> = BTreeSet::new();
        let mut unique_y: BTreeSet<u32> = BTreeSet::new();

        for record in &response.channel_records {
            let channel_idx = record.id.saturating_sub(1) as usize;
            if channel_idx < channel_count {
                if let Some(mux) = record.mux_records.first() {
                    raw_coords[channel_idx] = (mux.phys_x, mux.phys_y);
                    unique_x.insert(mux.phys_x);
                    unique_y.insert(mux.phys_y);
                }
            }
        }

        let x_map: std::collections::HashMap<u32, u32> = unique_x
            .iter()
            .enumerate()
            .map(|(idx, &val)| (val, idx as u32))
            .collect();
        let y_map: std::collections::HashMap<u32, u32> = unique_y
            .iter()
            .enumerate()
            .map(|(idx, &val)| (val, idx as u32))
            .collect();

        let coords: Vec<(u32, u32)> = raw_coords
            .iter()
            .map(|(x, y)| (*x_map.get(x).unwrap_or(&0), *y_map.get(y).unwrap_or(&0)))
            .collect();

        Ok(super::ChannelLayout {
            channel_count,
            width: unique_x.len() as u32,
            height: unique_y.len() as u32,
            coords,
        })
    }
}

impl std::fmt::Debug for PositionClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionClient")
            .field("position", &self.position.name)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct AcquisitionInfo {
    pub run_id: String,
    pub state: RunState,
    pub reads_processed: u64,
    pub reads_passed: u64,
    pub reads_failed: u64,
    pub bases_passed: u64,
    pub bases_failed: u64,
}
