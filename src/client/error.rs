//! Unified error type for all client operations.

use std::fmt;

/// Unified error type for all MinKNOW client operations.
///
/// This enum captures all possible error conditions when interacting
/// with MinKNOW services, providing consistent error handling across
/// the TUI and CLI.
#[derive(Debug)]
pub enum ClientError {
    /// Failed to establish connection to endpoint.
    Connection {
        endpoint: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// gRPC call failed.
    Grpc {
        method: String,
        status: tonic::Status,
    },

    /// Response parsing or validation failed.
    Protocol { message: String },

    /// Requested resource was not found.
    NotFound { resource: String, id: String },

    /// Operation timed out.
    Timeout { operation: String },

    /// Connection was lost.
    Disconnected,

    /// Failed to read or parse authentication token.
    Auth { message: String },
}

impl ClientError {
    /// Returns true if this error might be recoverable via retry.
    pub fn is_retriable(&self) -> bool {
        match self {
            ClientError::Connection { .. } => true,
            ClientError::Grpc { status, .. } => {
                use tonic::Code;
                matches!(
                    status.code(),
                    Code::Unavailable | Code::DeadlineExceeded | Code::Aborted
                )
            }
            ClientError::Timeout { .. } => true,
            ClientError::Disconnected => true,
            ClientError::Protocol { .. } => false,
            ClientError::NotFound { .. } => false,
            ClientError::Auth { .. } => false,
        }
    }

    /// Returns a human-readable error message suitable for display.
    pub fn display_message(&self) -> String {
        match self {
            ClientError::Connection { endpoint, .. } => {
                format!("Failed to connect to {}", endpoint)
            }
            ClientError::Grpc { method, status } => {
                format!("{}: {}", method, status.message())
            }
            ClientError::Protocol { message } => {
                format!("Protocol error: {}", message)
            }
            ClientError::NotFound { resource, id } => {
                format!("{} not found: {}", resource, id)
            }
            ClientError::Timeout { operation } => {
                format!("Operation timed out: {}", operation)
            }
            ClientError::Disconnected => "Connection lost".to_string(),
            ClientError::Auth { message } => {
                format!("Authentication error: {}", message)
            }
        }
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_message())
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClientError::Connection { source, .. } => Some(source.as_ref()),
            ClientError::Grpc { status, .. } => Some(status),
            _ => None,
        }
    }
}

impl From<tonic::Status> for ClientError {
    fn from(status: tonic::Status) -> Self {
        ClientError::Grpc {
            method: "unknown".into(),
            status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_is_retriable() {
        let err = ClientError::Connection {
            endpoint: "localhost:9501".into(),
            source: "connection refused".into(),
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_grpc_unavailable_is_retriable() {
        let err = ClientError::Grpc {
            method: "list_devices".into(),
            status: tonic::Status::unavailable("service unavailable"),
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_grpc_deadline_exceeded_is_retriable() {
        let err = ClientError::Grpc {
            method: "get_stats".into(),
            status: tonic::Status::deadline_exceeded("timeout"),
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_grpc_not_found_is_not_retriable() {
        let err = ClientError::Grpc {
            method: "get_device".into(),
            status: tonic::Status::not_found("device not found"),
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_timeout_is_retriable() {
        let err = ClientError::Timeout {
            operation: "connect".into(),
        };
        assert!(err.is_retriable());
    }

    #[test]
    fn test_disconnected_is_retriable() {
        let err = ClientError::Disconnected;
        assert!(err.is_retriable());
    }

    #[test]
    fn test_not_found_is_not_retriable() {
        let err = ClientError::NotFound {
            resource: "Device".into(),
            id: "DEV001".into(),
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_protocol_error_is_not_retriable() {
        let err = ClientError::Protocol {
            message: "unexpected response".into(),
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_auth_error_is_not_retriable() {
        let err = ClientError::Auth {
            message: "invalid token".into(),
        };
        assert!(!err.is_retriable());
    }

    #[test]
    fn test_display_messages() {
        let err = ClientError::Connection {
            endpoint: "localhost:9501".into(),
            source: "refused".into(),
        };
        assert_eq!(err.display_message(), "Failed to connect to localhost:9501");

        let err = ClientError::NotFound {
            resource: "Position".into(),
            id: "P1".into(),
        };
        assert_eq!(err.display_message(), "Position not found: P1");

        let err = ClientError::Disconnected;
        assert_eq!(err.display_message(), "Connection lost");

        let err = ClientError::Auth {
            message: "token expired".into(),
        };
        assert_eq!(err.display_message(), "Authentication error: token expired");
    }

    #[test]
    fn test_from_tonic_status() {
        let status = tonic::Status::internal("internal error");
        let err: ClientError = status.into();

        match err {
            ClientError::Grpc { method, status } => {
                assert_eq!(method, "unknown");
                assert_eq!(status.code(), tonic::Code::Internal);
            }
            _ => panic!("Expected Grpc error"),
        }
    }
}
