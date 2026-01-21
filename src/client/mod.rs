//! # Client Module
//!
//! This module provides the gRPC client for communicating with MinKNOW.
//!
//! ## Overview
//!
//! The main entry point is [`Client`], which handles connection management,
//! automatic reconnection, and provides typed wrappers for MinKNOW services.
//!
//! ## Architecture
//!
//! ```text
//! Client
//!   ├── ManagerClient     — Device/position discovery
//!   ├── AcquisitionClient — Run state and control (per-position)
//!   └── StatisticsClient  — Streaming metrics (per-position)
//! ```

mod error;
mod position;
mod types;

pub use error::ClientError;
pub use position::{AcquisitionInfo, PositionClient};
pub use types::{
    ChannelLayout, ChannelState, ChannelStatesSnapshot, Device, DeviceState, DutyTimeSnapshot,
    PoreCategory, PoreCounts, Position, PositionState, ReadLengthHistogram, RunState,
    StatsSnapshot, YieldDataPoint,
};

use crate::proto::minknow_api::manager::{
    manager_service_client::ManagerServiceClient, FlowCellPositionsRequest,
    LocalAuthenticationTokenPathRequest,
};
use rand::Rng;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

const MINKNOW_CA_CERT_PATH: &str = "/var/lib/minknow/data/rpc-certs/minknow/ca.crt";

#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter_fraction: f64,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter_fraction: 0.1,
            max_attempts: None,
        }
    }
}

impl ReconnectPolicy {
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay.as_secs_f64() * self.multiplier.powi(attempt as i32);
        let capped_delay = base_delay.min(self.max_delay.as_secs_f64());

        let jitter_range = capped_delay * self.jitter_fraction;
        let jitter = if jitter_range > 0.0 {
            rand::rng().random_range(-jitter_range..jitter_range)
        } else {
            0.0
        };
        let final_delay = (capped_delay + jitter).max(0.0);

        Duration::from_secs_f64(final_delay)
    }
}

pub struct Client {
    endpoint: String,
    host: String,
    manager: ManagerServiceClient<Channel>,
    auth_token: Option<Arc<str>>,
}

impl Client {
    pub async fn connect(host: &str, port: u16) -> Result<Self, ClientError> {
        let endpoint = format!("https://{}:{}", host, port);
        tracing::info!(endpoint = %endpoint, "Connecting to MinKNOW manager");

        let ca_cert_path = Path::new(MINKNOW_CA_CERT_PATH);
        if !ca_cert_path.exists() {
            return Err(ClientError::Connection {
                endpoint: endpoint.clone(),
                source: format!(
                    "MinKNOW CA certificate not found at {}. Is MinKNOW installed?",
                    MINKNOW_CA_CERT_PATH
                )
                .into(),
            });
        }

        let ca_cert =
            std::fs::read_to_string(ca_cert_path).map_err(|e| ClientError::Connection {
                endpoint: endpoint.clone(),
                source: Box::new(e),
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

        let mut manager = ManagerServiceClient::new(channel);

        let auth_token = Self::fetch_auth_token(&mut manager).await?;

        tracing::info!("Connected to MinKNOW manager");
        Ok(Self {
            endpoint,
            host: host.to_string(),
            manager,
            auth_token,
        })
    }

    async fn fetch_auth_token(
        manager: &mut ManagerServiceClient<Channel>,
    ) -> Result<Option<Arc<str>>, ClientError> {
        let response = manager
            .local_authentication_token_path(LocalAuthenticationTokenPathRequest {})
            .await
            .map_err(|status| ClientError::Grpc {
                method: "local_authentication_token_path".into(),
                status,
            })?
            .into_inner();

        if response.path.is_empty() {
            tracing::debug!("No local auth token path returned (guest mode enabled?)");
            return Ok(None);
        }

        let token_path = Path::new(&response.path);
        if !token_path.exists() {
            tracing::debug!(path = %response.path, "Auth token file does not exist");
            return Ok(None);
        }

        let content = std::fs::read_to_string(token_path).map_err(|e| ClientError::Auth {
            message: format!("Failed to read token file {}: {}", response.path, e),
        })?;

        let token_data: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| ClientError::Auth {
                message: format!("Failed to parse token file: {}", e),
            })?;

        let token = token_data
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::Auth {
                message: "Token file missing 'token' field".into(),
            })?;

        tracing::debug!("Loaded local authentication token");
        Ok(Some(Arc::from(token)))
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub async fn connect_position(
        &self,
        position: Position,
    ) -> Result<PositionClient, ClientError> {
        PositionClient::connect(position, &self.host, self.auth_token.clone()).await
    }

    pub async fn list_positions(&mut self) -> Result<Vec<Position>, ClientError> {
        tracing::debug!("Listing flow cell positions");

        let request = tonic::Request::new(FlowCellPositionsRequest {});

        let mut stream = self
            .manager
            .flow_cell_positions(request)
            .await
            .map_err(|status| ClientError::Grpc {
                method: "flow_cell_positions".into(),
                status,
            })?
            .into_inner();

        let mut positions = Vec::new();

        while let Some(response) = stream.message().await.map_err(|status| ClientError::Grpc {
            method: "flow_cell_positions".into(),
            status,
        })? {
            for pos in response.positions {
                positions.push(Position::from_proto(pos));
            }
        }

        tracing::debug!(count = positions.len(), "Found positions");
        Ok(positions)
    }

    pub async fn list_devices(&mut self) -> Result<Vec<Device>, ClientError> {
        let positions = self.list_positions().await?;

        let mut devices: std::collections::HashMap<String, Device> =
            std::collections::HashMap::new();

        for pos in &positions {
            if !devices.contains_key(&pos.device_id) {
                devices.insert(
                    pos.device_id.clone(),
                    Device {
                        id: pos.device_id.clone(),
                        name: pos.device_id.clone(),
                        state: if pos.state == PositionState::Running {
                            DeviceState::Ready
                        } else if pos.state == PositionState::Error {
                            DeviceState::Error
                        } else {
                            DeviceState::Ready
                        },
                    },
                );
            }
        }

        Ok(devices.into_values().collect())
    }
}

impl Client {
    pub async fn connect_with_retry(
        host: &str,
        port: u16,
        policy: &ReconnectPolicy,
    ) -> Result<Self, ClientError> {
        let mut attempt = 0u32;

        loop {
            match Self::connect(host, port).await {
                Ok(client) => return Ok(client),
                Err(e) if e.is_retriable() => {
                    if let Some(max) = policy.max_attempts {
                        if attempt >= max {
                            tracing::error!(
                                attempt,
                                max_attempts = max,
                                "Max reconnection attempts reached"
                            );
                            return Err(e);
                        }
                    }

                    let delay = policy.delay_for_attempt(attempt);
                    tracing::warn!(
                        attempt,
                        delay_ms = delay.as_millis(),
                        error = %e,
                        "Connection failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnect_policy_default() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert_eq!(policy.multiplier, 2.0);
        assert!(policy.max_attempts.is_none());
    }

    #[test]
    fn test_reconnect_policy_exponential_backoff() {
        let policy = ReconnectPolicy {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            jitter_fraction: 0.0,
            max_attempts: None,
        };

        let d0 = policy.delay_for_attempt(0);
        let d1 = policy.delay_for_attempt(1);
        let d2 = policy.delay_for_attempt(2);

        assert_eq!(d0, Duration::from_secs(1));
        assert_eq!(d1, Duration::from_secs(2));
        assert_eq!(d2, Duration::from_secs(4));
    }

    #[test]
    fn test_reconnect_policy_respects_max_delay() {
        let policy = ReconnectPolicy {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            multiplier: 10.0,
            jitter_fraction: 0.0,
            max_attempts: None,
        };

        let d2 = policy.delay_for_attempt(2);
        assert_eq!(d2, Duration::from_secs(5));
    }

    #[test]
    fn test_reconnect_policy_jitter_range() {
        let policy = ReconnectPolicy {
            initial_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(60),
            multiplier: 1.0,
            jitter_fraction: 0.1,
            max_attempts: None,
        };

        let mut min_seen = Duration::from_secs(100);
        let mut max_seen = Duration::from_secs(0);

        for _ in 0..100 {
            let d = policy.delay_for_attempt(0);
            if d < min_seen {
                min_seen = d;
            }
            if d > max_seen {
                max_seen = d;
            }
        }

        assert!(min_seen >= Duration::from_secs(9));
        assert!(max_seen <= Duration::from_secs(11));
        assert!(min_seen != max_seen);
    }
}
