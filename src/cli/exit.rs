//! Exit code definitions for CLI commands.
//!
//! Per SPEC_CLI.md:
//! - 0: Success
//! - 1: General error
//! - 2: Connection failed
//! - 3: Invalid arguments
//! - 4: Resource not found

use std::process::ExitCode;

use crate::client::ClientError;

/// Exit codes for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Exit {
    /// Success
    Ok = 0,
    /// General error
    Error = 1,
    /// Connection failed
    Connection = 2,
    /// Invalid arguments
    Args = 3,
    /// Resource not found (device/position)
    NotFound = 4,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        ExitCode::from(exit as u8)
    }
}

/// Maps an error to the appropriate exit code.
pub fn exit_code_for_error(err: &anyhow::Error) -> Exit {
    // Check if it's a ClientError
    if let Some(client_err) = err.downcast_ref::<ClientError>() {
        return match client_err {
            ClientError::Connection { .. } => Exit::Connection,
            ClientError::Disconnected => Exit::Connection,
            ClientError::Timeout { .. } => Exit::Connection,
            ClientError::NotFound { .. } => Exit::NotFound,
            ClientError::Auth { .. } => Exit::Connection, // Auth issues are connection-related
            ClientError::Grpc { status, .. } => {
                use tonic::Code;
                match status.code() {
                    Code::NotFound => Exit::NotFound,
                    Code::Unavailable | Code::DeadlineExceeded => Exit::Connection,
                    Code::InvalidArgument => Exit::Args,
                    _ => Exit::Error,
                }
            }
            ClientError::Protocol { .. } => Exit::Error,
        };
    }

    // Check for config/arg errors by message heuristics
    let msg = err.to_string().to_lowercase();
    if msg.contains("invalid") && (msg.contains("argument") || msg.contains("config")) {
        return Exit::Args;
    }
    if msg.contains("not found") {
        return Exit::NotFound;
    }
    if msg.contains("connection") || msg.contains("connect") {
        return Exit::Connection;
    }

    Exit::Error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_values() {
        assert_eq!(Exit::Ok as u8, 0);
        assert_eq!(Exit::Error as u8, 1);
        assert_eq!(Exit::Connection as u8, 2);
        assert_eq!(Exit::Args as u8, 3);
        assert_eq!(Exit::NotFound as u8, 4);
    }

    #[test]
    fn test_connection_error_maps_to_code_2() {
        let err = ClientError::Connection {
            endpoint: "localhost:9501".into(),
            source: "refused".into(),
        };
        let anyhow_err = anyhow::Error::new(err);
        assert_eq!(exit_code_for_error(&anyhow_err), Exit::Connection);
    }

    #[test]
    fn test_not_found_error_maps_to_code_4() {
        let err = ClientError::NotFound {
            resource: "Position".into(),
            id: "X1-A".into(),
        };
        let anyhow_err = anyhow::Error::new(err);
        assert_eq!(exit_code_for_error(&anyhow_err), Exit::NotFound);
    }

    #[test]
    fn test_timeout_maps_to_connection() {
        let err = ClientError::Timeout {
            operation: "connect".into(),
        };
        let anyhow_err = anyhow::Error::new(err);
        assert_eq!(exit_code_for_error(&anyhow_err), Exit::Connection);
    }

    #[test]
    fn test_grpc_not_found_maps_to_code_4() {
        let err = ClientError::Grpc {
            method: "get_position".into(),
            status: tonic::Status::not_found("position not found"),
        };
        let anyhow_err = anyhow::Error::new(err);
        assert_eq!(exit_code_for_error(&anyhow_err), Exit::NotFound);
    }

    #[test]
    fn test_grpc_unavailable_maps_to_connection() {
        let err = ClientError::Grpc {
            method: "list".into(),
            status: tonic::Status::unavailable("service down"),
        };
        let anyhow_err = anyhow::Error::new(err);
        assert_eq!(exit_code_for_error(&anyhow_err), Exit::Connection);
    }
}
