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
    ChannelLayout, ChannelState, ChannelStatesSnapshot, Device, DeviceState, DeviceType,
    DutyTimeSnapshot, FlowCellInfo, PoreCategory, PoreCounts, Position, PositionState,
    ReadLengthHistogram, RunInfo, RunState, StatsSnapshot, YieldDataPoint,
};

use crate::proto::minknow_api::manager::{
    manager_service_client::ManagerServiceClient, FlowCellPositionsRequest,
    LocalAuthenticationTokenPathRequest,
};
use rand::Rng;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

/// Default CA certificate search paths for Linux, in order of priority.
/// Matches the paths used by the official Python minknow_api library.
#[cfg(target_os = "linux")]
pub(crate) const MINKNOW_CA_CERT_PATHS: &[&str] = &[
    "/data/rpc-certs/minknow/ca.crt",
    "/var/lib/minknow/data/rpc-certs/minknow/ca.crt",
];

/// Default CA certificate search path for macOS.
#[cfg(target_os = "macos")]
pub(crate) const MINKNOW_CA_CERT_PATHS: &[&str] =
    &["/Library/MinKNOW/data/rpc-certs/minknow/ca.crt"];

/// Environment variable for custom CA certificate path (matches Python library).
pub(crate) const MINKNOW_TRUSTED_CA_ENV: &str = "MINKNOW_TRUSTED_CA";

pub(crate) const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn is_localhost(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

#[allow(clippy::result_large_err)]
pub(crate) fn tls_domain_for_host(endpoint: &str, host: &str) -> Result<&'static str, ClientError> {
    if is_localhost(host) {
        Ok("localhost")
    } else {
        Err(ClientError::Connection {
            endpoint: endpoint.to_string(),
            source: "Remote hosts are not supported; use localhost".into(),
        })
    }
}

/// Load the MinKNOW CA certificate for TLS connections.
///
/// Search order (matches official Python minknow_api library):
/// 1. `MINKNOW_TRUSTED_CA` environment variable (custom path)
/// 2. Platform-specific default paths:
///    - Linux: `/data/rpc-certs/minknow/ca.crt`, `/var/lib/minknow/data/rpc-certs/minknow/ca.crt`
///    - macOS: `/Library/MinKNOW/data/rpc-certs/minknow/ca.crt`
pub(crate) async fn load_ca_cert(endpoint: &str) -> Result<String, ClientError> {
    // First, check environment variable for custom cert path
    if let Ok(custom_path) = std::env::var(MINKNOW_TRUSTED_CA_ENV) {
        let path = Path::new(&custom_path);
        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                tracing::debug!(path = %custom_path, "Loaded CA cert from MINKNOW_TRUSTED_CA");
                return Ok(content);
            }
            Err(e) => {
                tracing::warn!(
                    path = %custom_path,
                    error = %e,
                    "MINKNOW_TRUSTED_CA set but failed to read certificate"
                );
                // Fall through to try default paths
            }
        }
    }

    // Try platform-specific default paths
    let mut last_error = None;
    for path_str in MINKNOW_CA_CERT_PATHS {
        let path = Path::new(path_str);
        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                tracing::debug!(path = %path_str, "Loaded CA cert from default path");
                return Ok(content);
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                tracing::trace!(path = %path_str, "CA cert not found, trying next path");
                continue;
            }
            Err(e) => {
                tracing::warn!(path = %path_str, error = %e, "Failed to read CA cert");
                last_error = Some(e);
            }
        }
    }

    // No cert found - provide helpful error message
    let paths_tried = MINKNOW_CA_CERT_PATHS.join(", ");
    Err(ClientError::Connection {
        endpoint: endpoint.to_string(),
        source: if let Some(e) = last_error {
            format!(
                "Failed to read MinKNOW CA certificate: {}. Paths tried: {}. \
                 You can set MINKNOW_TRUSTED_CA environment variable to specify a custom path.",
                e, paths_tried
            )
            .into()
        } else {
            format!(
                "MinKNOW CA certificate not found. Paths tried: {}. Is MinKNOW installed? \
                 You can set MINKNOW_TRUSTED_CA environment variable to specify a custom path.",
                paths_tried
            )
            .into()
        },
    })
}

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
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl Client {
    pub async fn connect(host: &str, port: u16) -> Result<Self, ClientError> {
        Self::connect_with_timeouts(host, port, DEFAULT_CONNECT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT)
            .await
    }

    pub async fn connect_with_timeouts(
        host: &str,
        port: u16,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, ClientError> {
        let endpoint = format!("https://{}:{}", host, port);
        tracing::info!(endpoint = %endpoint, "Connecting to MinKNOW manager");

        let tls_domain = tls_domain_for_host(&endpoint, host)?;
        let ca_cert = load_ca_cert(&endpoint).await?;

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

        let mut manager = ManagerServiceClient::new(channel);

        let auth_token = Self::fetch_auth_token(&mut manager).await?;

        tracing::info!("Connected to MinKNOW manager");
        Ok(Self {
            endpoint,
            host: host.to_string(),
            manager,
            auth_token,
            connect_timeout,
            request_timeout,
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
        let content = match tokio::fs::read_to_string(token_path).await {
            Ok(content) => content,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                tracing::debug!(path = %response.path, "Auth token file does not exist");
                return Ok(None);
            }
            Err(e) => {
                return Err(ClientError::Auth {
                    message: format!("Failed to read token file {}: {}", response.path, e),
                });
            }
        };

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
        PositionClient::connect_with_timeouts(
            position,
            &self.host,
            self.auth_token.clone(),
            self.connect_timeout,
            self.request_timeout,
        )
        .await
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
