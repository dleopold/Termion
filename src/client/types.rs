//! Domain types for MinKNOW data.
//!
//! These types provide a stable API separate from proto-generated types,
//! allowing internal changes without breaking consumers.

use crate::proto::minknow_api::manager::{flow_cell_position, FlowCellPosition};
use chrono::{DateTime, Utc};

/// A connected sequencing device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Unique device identifier (e.g., "MS00001").
    pub id: String,

    /// Human-readable device name.
    pub name: String,

    /// Current device state.
    pub state: DeviceState,
}

/// Device connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceState {
    /// Device is connected and ready.
    #[default]
    Ready,

    /// Device is busy with an operation.
    Busy,

    /// Device has an error condition.
    Error,

    /// Device is disconnected or unavailable.
    Offline,
}

/// A sequencing position on a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// Position identifier.
    pub id: String,

    /// Human-readable position name (e.g., "X1").
    pub name: String,

    /// Parent device ID.
    pub device_id: String,

    /// Current position state.
    pub state: PositionState,

    /// gRPC port for this position's services.
    pub grpc_port: u16,

    /// Whether this is a simulated device.
    pub is_simulated: bool,
}

impl Position {
    /// Convert from proto FlowCellPosition to domain Position.
    pub(crate) fn from_proto(proto: FlowCellPosition) -> Self {
        let state = match flow_cell_position::State::try_from(proto.state) {
            Ok(flow_cell_position::State::Initialising) => PositionState::Initializing,
            Ok(flow_cell_position::State::Running) => PositionState::Running,
            Ok(flow_cell_position::State::HardwareError)
            | Ok(flow_cell_position::State::SoftwareError) => PositionState::Error,
            _ => PositionState::Idle,
        };

        let grpc_port = proto
            .rpc_ports
            .as_ref()
            .map(|p| p.secure as u16)
            .unwrap_or(0);

        // Use parent_name if available, otherwise use position name
        let device_id = if proto.parent_name.is_empty() {
            proto.name.clone()
        } else {
            proto.parent_name
        };

        Self {
            id: proto.name.clone(),
            name: proto.name,
            device_id,
            state,
            grpc_port,
            is_simulated: proto.is_simulated,
        }
    }
}

/// Position state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionState {
    /// Position is idle, no run active.
    #[default]
    Idle,

    /// Position is initializing.
    Initializing,

    /// Position has an active run.
    Running,

    /// Position has an error condition.
    Error,
}

/// Acquisition run state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RunState {
    /// No acquisition in progress.
    #[default]
    Idle,

    /// Acquisition is starting up.
    Starting,

    /// Acquisition is actively running.
    Running,

    /// Acquisition is paused.
    Paused,

    /// Acquisition is finishing up.
    Finishing,

    /// Acquisition has stopped.
    Stopped,

    /// Acquisition encountered an error.
    Error(String),
}

impl RunState {
    /// Returns true if this state represents an active run.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            RunState::Starting | RunState::Running | RunState::Paused | RunState::Finishing
        )
    }

    /// Returns a short label for display.
    pub fn label(&self) -> &'static str {
        match self {
            RunState::Idle => "Idle",
            RunState::Starting => "Starting",
            RunState::Running => "Running",
            RunState::Paused => "Paused",
            RunState::Finishing => "Finishing",
            RunState::Stopped => "Stopped",
            RunState::Error(_) => "Error",
        }
    }
}

/// A snapshot of acquisition statistics.
#[derive(Debug, Clone, Default)]
pub struct StatsSnapshot {
    /// Timestamp of this snapshot.
    pub timestamp: Option<DateTime<Utc>>,

    /// Total reads processed.
    pub reads_processed: u64,

    /// Total bases called.
    pub bases_called: u64,

    /// Current throughput in bases per second.
    pub throughput_bps: f64,

    /// Estimated throughput in gigabases per hour.
    pub throughput_gbph: f64,

    /// Number of reads passing filters.
    pub reads_passed: u64,

    /// Number of reads failing filters.
    pub reads_failed: u64,

    /// Mean read quality score.
    pub mean_quality: f64,

    /// Mean read length in bases.
    pub mean_read_length: f64,

    /// Number of active pores.
    pub active_pores: u32,
}

impl StatsSnapshot {
    /// Returns the pass rate as a percentage (0.0 - 100.0).
    pub fn pass_rate(&self) -> f64 {
        let total = self.reads_passed + self.reads_failed;
        if total == 0 {
            0.0
        } else {
            (self.reads_passed as f64 / total as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_state_is_active() {
        assert!(!RunState::Idle.is_active());
        assert!(RunState::Starting.is_active());
        assert!(RunState::Running.is_active());
        assert!(RunState::Paused.is_active());
        assert!(RunState::Finishing.is_active());
        assert!(!RunState::Stopped.is_active());
        assert!(!RunState::Error("test".into()).is_active());
    }

    #[test]
    fn test_run_state_labels() {
        assert_eq!(RunState::Idle.label(), "Idle");
        assert_eq!(RunState::Starting.label(), "Starting");
        assert_eq!(RunState::Running.label(), "Running");
        assert_eq!(RunState::Paused.label(), "Paused");
        assert_eq!(RunState::Finishing.label(), "Finishing");
        assert_eq!(RunState::Stopped.label(), "Stopped");
        assert_eq!(RunState::Error("oops".into()).label(), "Error");
    }

    #[test]
    fn test_stats_pass_rate_zero_reads() {
        let stats = StatsSnapshot::default();
        assert_eq!(stats.pass_rate(), 0.0);
    }

    #[test]
    fn test_stats_pass_rate_all_passed() {
        let stats = StatsSnapshot {
            reads_passed: 100,
            reads_failed: 0,
            ..Default::default()
        };
        assert_eq!(stats.pass_rate(), 100.0);
    }

    #[test]
    fn test_stats_pass_rate_all_failed() {
        let stats = StatsSnapshot {
            reads_passed: 0,
            reads_failed: 100,
            ..Default::default()
        };
        assert_eq!(stats.pass_rate(), 0.0);
    }

    #[test]
    fn test_stats_pass_rate_mixed() {
        let stats = StatsSnapshot {
            reads_passed: 75,
            reads_failed: 25,
            ..Default::default()
        };
        assert_eq!(stats.pass_rate(), 75.0);
    }

    #[test]
    fn test_device_state_default() {
        assert_eq!(DeviceState::default(), DeviceState::Ready);
    }

    #[test]
    fn test_position_state_default() {
        assert_eq!(PositionState::default(), PositionState::Idle);
    }

    #[test]
    fn test_run_state_default() {
        assert_eq!(RunState::default(), RunState::Idle);
    }
}
