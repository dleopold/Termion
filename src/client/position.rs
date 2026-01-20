//! Position-specific client for acquisition and statistics services.

use super::{ClientError, Position, RunState, StatsSnapshot};
use crate::proto::minknow_api::acquisition::{
    acquisition_service_client::AcquisitionServiceClient, CurrentStatusRequest,
    GetAcquisitionRunInfoRequest, MinknowStatus, StopRequest,
};
use crate::proto::minknow_api::statistics::{
    statistics_service_client::StatisticsServiceClient, StreamAcquisitionOutputRequest,
};
use std::sync::Arc;
use std::time::Duration;
use tonic::service::Interceptor;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

const MINKNOW_CA_CERT_PATH: &str = "/var/lib/minknow/data/rpc-certs/minknow/ca.crt";

#[derive(Clone)]
struct AuthInterceptor {
    token: Option<Arc<str>>,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(ref token) = self.token {
            let value = token.parse().map_err(|_| {
                tonic::Status::internal("Invalid auth token format")
            })?;
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
}

impl PositionClient {
    pub async fn connect(
        position: Position,
        host: &str,
        auth_token: Option<Arc<str>>,
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

        let ca_cert = std::fs::read_to_string(MINKNOW_CA_CERT_PATH).map_err(|e| {
            ClientError::Connection {
                endpoint: endpoint.clone(),
                source: Box::new(e),
            }
        })?;

        let tls_config = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(&ca_cert))
            .domain_name("localhost");

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
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .connect()
            .await
            .map_err(|e| ClientError::Connection {
                endpoint: endpoint.clone(),
                source: Box::new(e),
            })?;

        let interceptor = AuthInterceptor { token: auth_token };
        let acquisition =
            AcquisitionServiceClient::with_interceptor(channel.clone(), interceptor.clone());
        let statistics = StatisticsServiceClient::with_interceptor(channel, interceptor);

        tracing::info!(position = %position.name, "Connected to position services");
        Ok(Self {
            position,
            acquisition,
            statistics,
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

        let state = match MinknowStatus::try_from(response.status) {
            Ok(MinknowStatus::Ready) => RunState::Idle,
            Ok(MinknowStatus::Starting) => RunState::Starting,
            Ok(MinknowStatus::Processing) => RunState::Running,
            Ok(MinknowStatus::Finishing) => RunState::Finishing,
            Ok(MinknowStatus::ErrorStatus) => RunState::Error("Unknown error".to_string()),
            Err(_) => RunState::Idle,
        };

        Ok(state)
    }

    pub async fn get_acquisition_info(&mut self) -> Result<AcquisitionInfo, ClientError> {
        let response = self
            .acquisition
            .get_acquisition_info(GetAcquisitionRunInfoRequest::default())
            .await
            .map_err(|status| ClientError::Grpc {
                method: "get_acquisition_info".into(),
                status,
            })?
            .into_inner();

        let state = match MinknowStatus::try_from(response.state) {
            Ok(MinknowStatus::Ready) => RunState::Idle,
            Ok(MinknowStatus::Starting) => RunState::Starting,
            Ok(MinknowStatus::Processing) => RunState::Running,
            Ok(MinknowStatus::Finishing) => RunState::Finishing,
            Ok(MinknowStatus::ErrorStatus) => RunState::Error("Unknown error".to_string()),
            Err(_) => RunState::Idle,
        };

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

        Ok(StatsSnapshot {
            timestamp: None,
            reads_processed: info.reads_processed,
            bases_called: info.bases_passed + info.bases_failed,
            throughput_bps: 0.0,
            throughput_gbph: 0.0,
            reads_passed: info.reads_passed,
            reads_failed: info.reads_failed,
            mean_quality: 0.0,
            mean_read_length: 0.0,
            active_pores: 0,
        })
    }

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
                                snapshot.bases_called = (yield_summary.basecalled_pass_bases
                                    + yield_summary.basecalled_fail_bases)
                                    as u64;
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
