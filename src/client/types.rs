//! Domain types for MinKNOW data.
//!
//! These types provide a stable API separate from proto-generated types,
//! allowing internal changes without breaking consumers.

use crate::proto::minknow_api::manager::{flow_cell_position, FlowCellPosition};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// A connected sequencing device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    /// Unique device identifier (e.g., "MS00001").
    pub id: String,

    /// Human-readable device name.
    pub name: String,

    /// Current device state.
    pub state: DeviceState,
}

/// Device connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
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

    /// Performing a mux scan (pore scan).
    MuxScanning,

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
            RunState::Starting
                | RunState::Running
                | RunState::MuxScanning
                | RunState::Paused
                | RunState::Finishing
        )
    }

    /// Returns a short label for display.
    pub fn label(&self) -> &'static str {
        match self {
            RunState::Idle => "Idle",
            RunState::Starting => "Starting",
            RunState::Running => "Running",
            RunState::MuxScanning => "Pore Scan",
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

    /// Number of bases passing filters.
    pub bases_passed: u64,

    /// Number of bases failing filters.
    pub bases_failed: u64,

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

/// A time-series data point for yield tracking.
#[derive(Debug, Clone, Default)]
pub struct YieldDataPoint {
    /// Seconds since start of acquisition.
    pub seconds: u32,
    /// Cumulative total read count at this time.
    pub reads: u64,
    /// Cumulative total bases at this time.
    pub bases: u64,
    /// Cumulative passed read count at this time.
    pub reads_passed: u64,
    /// Cumulative failed read count at this time.
    pub reads_failed: u64,
    /// Cumulative passed bases at this time.
    pub bases_passed: u64,
    /// Cumulative failed bases at this time.
    pub bases_failed: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ReadLengthHistogram {
    pub bucket_ranges: Vec<(u64, u64)>,
    pub bucket_values: Vec<u64>,
    pub n50: f32,
    pub outliers_excluded: bool,
    pub outlier_percent: f32,
    /// The requested range (min, max) if a specific range was requested, None for full range.
    pub requested_range: Option<(u64, u64)>,
    /// The actual data range end (source_data_end from response).
    pub source_data_end: u64,
}

impl ReadLengthHistogram {
    /// Returns the maximum bucket value for scaling.
    pub fn max_value(&self) -> u64 {
        self.bucket_values.iter().copied().max().unwrap_or(0)
    }

    /// Returns the total count across all buckets.
    pub fn total_count(&self) -> u64 {
        self.bucket_values.iter().sum()
    }
}

/// Channel state for duty time tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelState {
    /// Pore is sequencing (strand moving through).
    Strand,
    /// Pore is available but waiting.
    Pore,
    /// Adapter detected.
    Adapter,
    /// Channel is unavailable/blocked.
    Unavailable,
    /// Channel in recovery/unblock.
    Unblock,
    /// Other/unknown state.
    Other,
}

impl ChannelState {
    /// Returns a display label for this state.
    pub fn label(&self) -> &'static str {
        match self {
            ChannelState::Strand => "Sequencing",
            ChannelState::Pore => "Pore",
            ChannelState::Adapter => "Adapter",
            ChannelState::Unavailable => "Unavailable",
            ChannelState::Unblock => "Unblock",
            ChannelState::Other => "Other",
        }
    }

    /// Returns a suggested color index for this state (ANSI color codes).
    pub fn color_index(&self) -> u8 {
        match self {
            ChannelState::Strand => 2,
            ChannelState::Pore => 4,
            ChannelState::Adapter => 3,
            ChannelState::Unavailable => 1,
            ChannelState::Unblock => 5,
            ChannelState::Other => 8,
        }
    }
}

/// Duty time data for channel states.
#[derive(Debug, Clone, Default)]
pub struct DutyTimeSnapshot {
    /// Time range for this bucket (start, end) in seconds.
    pub time_range: (u32, u32),
    /// Map of channel state to time spent in that state (in samples).
    pub state_times: std::collections::HashMap<ChannelState, u64>,
    /// Pore occupancy values per channel (0.0-1.0).
    pub pore_occupancy: Vec<f32>,
}

/// Pore category based on occupancy level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoreCategory {
    Sequencing,
    PoreAvailable,
    Inactive,
    Unavailable,
}

impl PoreCategory {
    pub fn from_occupancy(occupancy: f32) -> Self {
        if occupancy >= 0.2 {
            PoreCategory::Sequencing
        } else if occupancy >= 0.05 {
            PoreCategory::PoreAvailable
        } else if occupancy > 0.0 {
            PoreCategory::Inactive
        } else {
            PoreCategory::Unavailable
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PoreCategory::Sequencing => "Sequencing",
            PoreCategory::PoreAvailable => "Pore Available",
            PoreCategory::Inactive => "Inactive",
            PoreCategory::Unavailable => "Unavailable",
        }
    }

    pub fn color_index(&self) -> u8 {
        match self {
            PoreCategory::Sequencing => 2,    // Green
            PoreCategory::PoreAvailable => 4, // Blue
            PoreCategory::Inactive => 8,      // Gray
            PoreCategory::Unavailable => 0,   // Black/Dark
        }
    }
}

/// Counts of pores in each category.
#[derive(Debug, Clone, Default)]
pub struct PoreCounts {
    pub sequencing: usize,
    pub pore_available: usize,
    pub inactive: usize,
    pub unavailable: usize,
}

impl PoreCounts {
    pub fn total(&self) -> usize {
        self.sequencing + self.pore_available + self.inactive + self.unavailable
    }
}

impl DutyTimeSnapshot {
    pub fn total_pores(&self) -> usize {
        self.pore_occupancy.len()
    }

    pub fn active_pores(&self, threshold: f32) -> usize {
        self.pore_occupancy
            .iter()
            .filter(|&&occ| occ > threshold)
            .count()
    }

    pub fn average_occupancy(&self) -> f32 {
        if self.pore_occupancy.is_empty() {
            0.0
        } else {
            self.pore_occupancy.iter().sum::<f32>() / self.pore_occupancy.len() as f32
        }
    }

    pub fn pore_counts(&self) -> PoreCounts {
        let mut counts = PoreCounts::default();
        for &occupancy in &self.pore_occupancy {
            match PoreCategory::from_occupancy(occupancy) {
                PoreCategory::Sequencing => counts.sequencing += 1,
                PoreCategory::PoreAvailable => counts.pore_available += 1,
                PoreCategory::Inactive => counts.inactive += 1,
                PoreCategory::Unavailable => counts.unavailable += 1,
            }
        }
        counts
    }

    pub fn state_fractions(&self) -> std::collections::HashMap<ChannelState, f64> {
        let total: u64 = self.state_times.values().sum();
        if total == 0 {
            return std::collections::HashMap::new();
        }
        self.state_times
            .iter()
            .map(|(&state, &time)| (state, time as f64 / total as f64))
            .collect()
    }
}

/// Physical layout of channels on the flow cell.
#[derive(Debug, Clone, Default)]
pub struct ChannelLayout {
    pub channel_count: usize,
    pub width: u32,
    pub height: u32,
    /// (x, y) coordinates for each channel, indexed by channel number (0-based).
    pub coords: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct ChannelStatesSnapshot {
    pub channel_count: usize,
    pub states: Vec<String>,
    pub state_counts: std::collections::HashMap<String, usize>,
}

impl ChannelStatesSnapshot {
    pub fn sequencing_count(&self) -> usize {
        self.state_counts
            .iter()
            .filter(|(name, _)| {
                let n = name.to_lowercase();
                // Only "strand" states represent actively sequencing channels
                // "pore" means an open pore NOT currently capturing DNA
                n.contains("strand") || n.contains("sequencing")
            })
            .map(|(_, count)| count)
            .sum()
    }

    pub fn pore_available_count(&self) -> usize {
        self.state_counts
            .iter()
            .filter(|(name, _)| {
                let n = name.to_lowercase();
                n.contains("single_pore") || n.contains("pore_available") || n == "pore"
            })
            .map(|(_, count)| count)
            .sum()
    }

    pub fn unavailable_count(&self) -> usize {
        self.state_counts
            .iter()
            .filter(|(name, _)| {
                let n = name.to_lowercase();
                // Unavailable: stalled strand or current too high
                n.contains("unavailable") || n.contains("saturated")
            })
            .map(|(_, count)| count)
            .sum()
    }

    pub fn inactive_count(&self) -> usize {
        self.state_counts
            .iter()
            .filter(|(name, _)| {
                let n = name.to_lowercase();
                // Inactive: cannot be rescued (outside detector limit, multiple pores)
                n.contains("inactive") || n.contains("zero") || n.contains("multiple")
            })
            .map(|(_, count)| count)
            .sum()
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
        assert!(RunState::MuxScanning.is_active());
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
        assert_eq!(RunState::MuxScanning.label(), "Pore Scan");
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
