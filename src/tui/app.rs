//! Application state and core logic.

use super::theme::Theme;
use crate::client::{
    ChannelLayout, ChannelStatesSnapshot, DutyTimeSnapshot, FlowCellInfo, Position,
    ReadLengthHistogram, RunInfo, RunState, StatsSnapshot, YieldDataPoint,
};
use crate::config::Config;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Overview,
    PositionDetail { position_idx: usize },
}

/// Which chart to display in the position detail view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailChart {
    /// Cumulative yield over time (reads or bases).
    #[default]
    Yield,
    /// Read length histogram.
    ReadLength,
    /// Pore activity visualization.
    PoreActivity,
}

/// Unit for yield chart display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YieldUnit {
    Bases,
    #[default]
    Reads,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunControlAction {
    Pause,
    Resume,
    Stop,
}

impl RunControlAction {
    pub fn label(&self) -> &'static str {
        match self {
            RunControlAction::Pause => "Pause",
            RunControlAction::Resume => "Resume",
            RunControlAction::Stop => "Stop",
        }
    }

    pub fn confirmation_message(&self) -> &'static str {
        match self {
            RunControlAction::Pause => "Pause the current run?",
            RunControlAction::Resume => "Resume the paused run?",
            RunControlAction::Stop => "Stop the current run? This cannot be undone.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    None,
    Help,
    Error {
        message: String,
    },
    RangeInput {
        max_input: String,
    },
    Confirmation {
        action: RunControlAction,
        position_name: String,
    },
    ThemeSelector {
        selected: usize,
    },
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connected,
    Connecting,
    Disconnected { since: Instant, reason: String },
    Reconnecting { attempt: u32 },
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub screen: Screen,
    pub overlay: Overlay,
    pub connection: ConnectionState,
    pub positions: Vec<Position>,
    pub selected_position: usize,
    pub stats_cache: HashMap<String, StatsSnapshot>,
    pub run_states: HashMap<String, RunState>,
    pub chart_data: HashMap<String, ChartBuffer>,
    pub should_quit: bool,
    pub last_error: Option<String>,
    pub detail_chart: DetailChart,
    pub yield_unit: YieldUnit,
    pub exclude_outliers: bool,
    /// Optional user-specified range for read length histogram (min, max) in bases.
    /// When set, the API will be queried with this range to get finer bins.
    pub histogram_range: Option<(u64, u64)>,
    pub yield_history: HashMap<String, Vec<YieldDataPoint>>,
    pub histograms: HashMap<String, ReadLengthHistogram>,
    pub duty_time: HashMap<String, DutyTimeSnapshot>,
    pub channel_states: HashMap<String, ChannelStatesSnapshot>,
    pub channel_layouts: HashMap<String, ChannelLayout>,
    pub run_info: HashMap<String, RunInfo>,
    pub flow_cell_info: HashMap<String, FlowCellInfo>,
    /// Tracks when throughput was last calculated for each position (for throttling).
    pub throughput_last_calc: HashMap<String, Instant>,
    pub channel_map_scroll_offset: usize,
}

pub struct ChartBuffer {
    pub data: VecDeque<(f64, f64)>,
    pub max_points: usize,
}

impl ChartBuffer {
    pub fn new(max_points: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_points),
            max_points,
        }
    }

    pub fn push(&mut self, timestamp: f64, value: f64) {
        if self.data.len() >= self.max_points {
            self.data.pop_front();
        }
        self.data.push_back((timestamp, value));
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl App {
    pub fn new(config: Config) -> Self {
        let theme = Theme::by_name(&config.tui.theme).unwrap_or_default();
        Self {
            config,
            theme,
            screen: Screen::Overview,
            overlay: Overlay::None,
            connection: ConnectionState::Connecting,
            positions: Vec::new(),
            selected_position: 0,
            stats_cache: HashMap::new(),
            run_states: HashMap::new(),
            chart_data: HashMap::new(),
            should_quit: false,
            last_error: None,
            detail_chart: DetailChart::default(),
            yield_unit: YieldUnit::default(),
            exclude_outliers: true,
            histogram_range: None,
            yield_history: HashMap::new(),
            histograms: HashMap::new(),
            duty_time: HashMap::new(),
            channel_states: HashMap::new(),
            channel_layouts: HashMap::new(),
            run_info: HashMap::new(),
            flow_cell_info: HashMap::new(),
            throughput_last_calc: HashMap::new(),
            channel_map_scroll_offset: 0,
        }
    }

    pub fn update_flow_cell_info(&mut self, position_name: &str, info: FlowCellInfo) {
        self.flow_cell_info.insert(position_name.to_string(), info);
    }

    pub fn update_channel_states(&mut self, position_name: &str, states: ChannelStatesSnapshot) {
        self.channel_states
            .insert(position_name.to_string(), states);
    }

    pub fn update_channel_layout(&mut self, position_name: &str, layout: ChannelLayout) {
        self.channel_layouts
            .insert(position_name.to_string(), layout);
    }

    pub fn select_next(&mut self) {
        if !self.positions.is_empty() {
            self.selected_position = (self.selected_position + 1) % self.positions.len();
            self.reset_channel_map_scroll();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.positions.is_empty() {
            self.selected_position = self
                .selected_position
                .checked_sub(1)
                .unwrap_or(self.positions.len() - 1);
            self.reset_channel_map_scroll();
        }
    }

    pub fn enter_detail(&mut self) {
        if let Some(pos) = self.positions.get(self.selected_position) {
            let is_active = self
                .run_states
                .get(&pos.name)
                .map(|s| s.is_active())
                .unwrap_or(false);
            if is_active {
                self.screen = Screen::PositionDetail {
                    position_idx: self.selected_position,
                };
            }
        }
    }

    pub fn back(&mut self) {
        match self.overlay {
            Overlay::None => {
                if self.screen != Screen::Overview {
                    self.screen = Screen::Overview;
                }
            }
            _ => {
                self.overlay = Overlay::None;
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.overlay = match self.overlay {
            Overlay::Help => Overlay::None,
            _ => Overlay::Help,
        };
    }

    pub fn open_theme_selector(&mut self) {
        let current_idx = Theme::available_themes()
            .iter()
            .position(|&name| Theme::by_name(name).map(|t| t.name) == Some(self.theme.name))
            .unwrap_or(0);
        self.overlay = Overlay::ThemeSelector {
            selected: current_idx,
        };
    }

    pub fn theme_selector_up(&mut self) {
        if let Overlay::ThemeSelector { selected } = &mut self.overlay {
            let count = Theme::available_themes().len();
            *selected = selected.checked_sub(1).unwrap_or(count - 1);
        }
    }

    pub fn theme_selector_down(&mut self) {
        if let Overlay::ThemeSelector { selected } = &mut self.overlay {
            let count = Theme::available_themes().len();
            *selected = (*selected + 1) % count;
        }
    }

    pub fn apply_selected_theme(&mut self) {
        if let Overlay::ThemeSelector { selected } = &self.overlay {
            let themes = Theme::available_themes();
            if let Some(&name) = themes.get(*selected) {
                if let Some(theme) = Theme::by_name(name) {
                    self.theme = theme;
                    if let Err(e) = Config::save_theme(name) {
                        tracing::warn!(error = %e, "Failed to save theme preference");
                    }
                }
            }
            self.overlay = Overlay::None;
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn set_error(&mut self, message: String) {
        self.last_error = Some(message.clone());
        self.overlay = Overlay::Error { message };
    }

    pub fn set_connected(&mut self) {
        self.connection = ConnectionState::Connected;
    }

    pub fn set_disconnected(&mut self, reason: String) {
        self.connection = ConnectionState::Disconnected {
            since: Instant::now(),
            reason,
        };
    }

    pub fn set_reconnecting(&mut self, attempt: u32) {
        self.connection = ConnectionState::Reconnecting { attempt };
    }

    pub fn update_positions(&mut self, positions: Vec<Position>) {
        self.positions = positions;
        if self.selected_position >= self.positions.len() && !self.positions.is_empty() {
            self.selected_position = self.positions.len() - 1;
        }
    }

    pub fn update_stats(&mut self, position_name: &str, mut stats: StatsSnapshot) {
        // Preserve existing throughput values (calculated separately from yield history)
        if let Some(existing) = self.stats_cache.get(position_name) {
            stats.throughput_bps = existing.throughput_bps;
            stats.throughput_gbph = existing.throughput_gbph;
        }
        self.stats_cache.insert(position_name.to_string(), stats);
    }

    pub fn update_run_state(&mut self, position_name: &str, state: RunState) {
        if !state.is_active() {
            self.stats_cache.remove(position_name);
            self.yield_history.remove(position_name);
            self.histograms.remove(position_name);
            self.duty_time.remove(position_name);
            self.channel_states.remove(position_name);
            self.run_info.remove(position_name);
        }
        self.run_states.insert(position_name.to_string(), state);
    }

    pub fn update_run_info(&mut self, position_name: &str, info: RunInfo) {
        self.run_info.insert(position_name.to_string(), info);
    }

    pub fn get_run_state(&self, position_name: &str) -> Option<&RunState> {
        self.run_states.get(position_name)
    }

    pub fn selected_position(&self) -> Option<&Position> {
        self.positions.get(self.selected_position)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionState::Connected)
    }

    pub fn cycle_detail_chart(&mut self) {
        self.detail_chart = match self.detail_chart {
            DetailChart::Yield => DetailChart::ReadLength,
            DetailChart::ReadLength => DetailChart::PoreActivity,
            DetailChart::PoreActivity => DetailChart::Yield,
        };
        if self.detail_chart != DetailChart::PoreActivity {
            self.reset_channel_map_scroll();
        }
    }

    pub fn set_detail_chart(&mut self, chart: DetailChart) {
        self.detail_chart = chart;
        if chart != DetailChart::PoreActivity {
            self.reset_channel_map_scroll();
        }
    }

    pub fn toggle_yield_unit(&mut self) {
        self.yield_unit = match self.yield_unit {
            YieldUnit::Bases => YieldUnit::Reads,
            YieldUnit::Reads => YieldUnit::Bases,
        };
        tracing::debug!(new_unit = ?self.yield_unit, "Toggled yield unit");
    }

    pub fn toggle_outliers(&mut self) {
        self.exclude_outliers = !self.exclude_outliers;
        tracing::info!(exclude_outliers = self.exclude_outliers, "Toggled outliers");
    }

    pub fn set_histogram_range(&mut self, min: u64, max: u64) {
        if min < max {
            self.histogram_range = Some((min, max));
            tracing::info!(min, max, "Set histogram range");
        }
    }

    pub fn clear_histogram_range(&mut self) {
        self.histogram_range = None;
    }

    pub fn has_histogram_range(&self) -> bool {
        self.histogram_range.is_some()
    }

    pub fn open_range_input(&mut self) {
        if self.detail_chart != DetailChart::ReadLength {
            return;
        }

        let max_str = match self.histogram_range {
            Some((_, max)) => max.to_string(),
            None => String::new(),
        };

        self.overlay = Overlay::RangeInput { max_input: max_str };
    }

    pub fn request_run_control(&mut self, action: RunControlAction) {
        let Some(pos) = self.selected_position() else {
            return;
        };

        let run_state = self.run_states.get(&pos.name);

        let is_valid = match action {
            RunControlAction::Pause => matches!(run_state, Some(RunState::Running)),
            RunControlAction::Resume => matches!(run_state, Some(RunState::Paused)),
            RunControlAction::Stop => {
                matches!(run_state, Some(RunState::Running | RunState::MuxScanning))
            }
        };

        if is_valid {
            self.overlay = Overlay::Confirmation {
                action,
                position_name: pos.name.clone(),
            };
        }
    }

    pub fn pending_confirmation(&self) -> Option<(RunControlAction, String)> {
        if let Overlay::Confirmation {
            action,
            position_name,
        } = &self.overlay
        {
            Some((*action, position_name.clone()))
        } else {
            None
        }
    }

    pub fn clear_confirmation(&mut self) {
        if matches!(self.overlay, Overlay::Confirmation { .. }) {
            self.overlay = Overlay::None;
        }
    }

    pub fn apply_range_input(&mut self) -> bool {
        if let Overlay::RangeInput { ref max_input, .. } = self.overlay {
            let max_input = max_input.clone();

            if max_input.is_empty() {
                self.histogram_range = None;
                self.overlay = Overlay::None;
                return true;
            }

            if let Ok(max_val) = max_input.parse::<u64>() {
                if max_val > 0 {
                    self.histogram_range = Some((0, max_val));
                    self.overlay = Overlay::None;
                    tracing::info!(max = max_val, "Applied histogram max range");
                    return true;
                }
            }
            tracing::warn!("Invalid range input");
            return false;
        }
        false
    }

    pub fn handle_range_input_key(&mut self, key_code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        if let Overlay::RangeInput { ref mut max_input } = self.overlay {
            match key_code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    max_input.push(c);
                }
                KeyCode::Backspace => {
                    max_input.pop();
                }
                KeyCode::Up => {
                    let current: u64 = max_input.parse().unwrap_or(0);
                    *max_input = (current + 1000).to_string();
                }
                KeyCode::Down => {
                    let current: u64 = max_input.parse().unwrap_or(0);
                    *max_input = current.saturating_sub(1000).to_string();
                }
                _ => {}
            }
        }
    }

    pub fn update_yield_history(&mut self, position_name: &str, data: Vec<YieldDataPoint>) {
        self.yield_history.insert(position_name.to_string(), data);
    }

    pub fn update_histogram(&mut self, position_name: &str, histogram: ReadLengthHistogram) {
        self.histograms.insert(position_name.to_string(), histogram);
    }

    pub fn update_duty_time(&mut self, position_name: &str, duty_time: DutyTimeSnapshot) {
        self.duty_time.insert(position_name.to_string(), duty_time);
    }

    /// Returns true if throughput should be calculated for this position.
    /// Throttles calculation to once every 5 seconds per position.
    pub fn should_calc_throughput(&self, position: &str) -> bool {
        self.throughput_last_calc
            .get(position)
            .map(|t| t.elapsed() >= std::time::Duration::from_secs(5))
            .unwrap_or(true)
    }

    /// Marks that throughput was just calculated for this position.
    pub fn mark_throughput_calculated(&mut self, position: &str) {
        self.throughput_last_calc
            .insert(position.to_string(), Instant::now());
    }

    /// Resets the channel map scroll offset to the top
    pub fn reset_channel_map_scroll(&mut self) {
        self.channel_map_scroll_offset = 0;
    }

    /// Clamps the channel map scroll offset to valid range
    pub fn clamp_channel_map_scroll(&mut self, total_rows: usize, visible_rows: usize) {
        let max_offset = total_rows.saturating_sub(visible_rows);
        self.channel_map_scroll_offset = self.channel_map_scroll_offset.min(max_offset);
    }

    /// Returns true if channel map scrolling should be active
    pub fn should_scroll_channel_map(&self) -> bool {
        matches!(self.screen, Screen::PositionDetail { .. })
            && self.detail_chart == DetailChart::PoreActivity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{DeviceType, PositionState};

    fn test_config() -> Config {
        Config::default()
    }

    fn test_position(name: &str) -> Position {
        Position {
            id: name.to_string(),
            name: name.to_string(),
            device_id: "DEV001".to_string(),
            state: PositionState::Idle,
            grpc_port: 8000,
            is_simulated: false,
            device_type: DeviceType::MinION,
        }
    }

    #[test]
    fn test_chart_buffer_new() {
        let buf = ChartBuffer::new(100);
        assert!(buf.data.is_empty());
        assert_eq!(buf.max_points, 100);
    }

    #[test]
    fn test_chart_buffer_push() {
        let mut buf = ChartBuffer::new(3);
        buf.push(1.0, 10.0);
        buf.push(2.0, 20.0);
        assert_eq!(buf.data.len(), 2);
        assert_eq!(buf.data[0], (1.0, 10.0));
        assert_eq!(buf.data[1], (2.0, 20.0));
    }

    #[test]
    fn test_chart_buffer_overflow() {
        let mut buf = ChartBuffer::new(3);
        buf.push(1.0, 10.0);
        buf.push(2.0, 20.0);
        buf.push(3.0, 30.0);
        buf.push(4.0, 40.0);
        assert_eq!(buf.data.len(), 3);
        assert_eq!(buf.data[0], (2.0, 20.0));
        assert_eq!(buf.data[2], (4.0, 40.0));
    }

    #[test]
    fn test_chart_buffer_clear() {
        let mut buf = ChartBuffer::new(10);
        buf.push(1.0, 10.0);
        buf.push(2.0, 20.0);
        buf.clear();
        assert!(buf.data.is_empty());
    }

    #[test]
    fn test_app_initial_state() {
        let app = App::new(test_config());
        assert_eq!(app.screen, Screen::Overview);
        assert_eq!(app.overlay, Overlay::None);
        assert!(app.positions.is_empty());
        assert_eq!(app.selected_position, 0);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_select_next_empty() {
        let mut app = App::new(test_config());
        app.select_next();
        assert_eq!(app.selected_position, 0);
    }

    #[test]
    fn test_select_next_wraps() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A"), test_position("B"), test_position("C")];
        app.selected_position = 2;
        app.select_next();
        assert_eq!(app.selected_position, 0);
    }

    #[test]
    fn test_select_previous_empty() {
        let mut app = App::new(test_config());
        app.select_previous();
        assert_eq!(app.selected_position, 0);
    }

    #[test]
    fn test_select_previous_wraps() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A"), test_position("B"), test_position("C")];
        app.selected_position = 0;
        app.select_previous();
        assert_eq!(app.selected_position, 2);
    }

    #[test]
    fn test_enter_detail_with_run() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A")];
        app.run_states.insert("A".to_string(), RunState::Running);
        app.enter_detail();
        assert_eq!(app.screen, Screen::PositionDetail { position_idx: 0 });
    }

    #[test]
    fn test_enter_detail_no_run() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A")];
        app.enter_detail();
        assert_eq!(app.screen, Screen::Overview);
    }

    #[test]
    fn test_enter_detail_empty() {
        let mut app = App::new(test_config());
        app.enter_detail();
        assert_eq!(app.screen, Screen::Overview);
    }

    #[test]
    fn test_back_from_detail() {
        let mut app = App::new(test_config());
        app.screen = Screen::PositionDetail { position_idx: 0 };
        app.back();
        assert_eq!(app.screen, Screen::Overview);
    }

    #[test]
    fn test_back_closes_overlay() {
        let mut app = App::new(test_config());
        app.overlay = Overlay::Help;
        app.back();
        assert_eq!(app.overlay, Overlay::None);
    }

    #[test]
    fn test_toggle_help() {
        let mut app = App::new(test_config());
        app.toggle_help();
        assert_eq!(app.overlay, Overlay::Help);
        app.toggle_help();
        assert_eq!(app.overlay, Overlay::None);
    }

    #[test]
    fn test_quit() {
        let mut app = App::new(test_config());
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn test_set_error() {
        let mut app = App::new(test_config());
        app.set_error("Something went wrong".into());
        assert_eq!(app.last_error, Some("Something went wrong".into()));
        assert_eq!(
            app.overlay,
            Overlay::Error {
                message: "Something went wrong".into()
            }
        );
    }

    #[test]
    fn test_update_positions_adjusts_selection() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A"), test_position("B"), test_position("C")];
        app.selected_position = 2;
        app.update_positions(vec![test_position("X")]);
        assert_eq!(app.selected_position, 0);
    }

    #[test]
    fn test_selected_position() {
        let mut app = App::new(test_config());
        assert!(app.selected_position().is_none());

        app.positions = vec![test_position("A"), test_position("B")];
        app.selected_position = 1;
        assert_eq!(app.selected_position().unwrap().name, "B");
    }

    #[test]
    fn test_is_connected() {
        let mut app = App::new(test_config());
        app.connection = ConnectionState::Connected;
        assert!(app.is_connected());

        app.set_disconnected("test".into());
        assert!(!app.is_connected());
    }

    #[test]
    fn test_cycle_detail_chart() {
        let mut app = App::new(test_config());
        assert_eq!(app.detail_chart, DetailChart::Yield);

        app.cycle_detail_chart();
        assert_eq!(app.detail_chart, DetailChart::ReadLength);

        app.cycle_detail_chart();
        assert_eq!(app.detail_chart, DetailChart::PoreActivity);

        app.cycle_detail_chart();
        assert_eq!(app.detail_chart, DetailChart::Yield);
    }

    #[test]
    fn test_set_detail_chart() {
        let mut app = App::new(test_config());
        app.set_detail_chart(DetailChart::PoreActivity);
        assert_eq!(app.detail_chart, DetailChart::PoreActivity);
    }

    #[test]
    fn test_toggle_yield_unit() {
        let mut app = App::new(test_config());
        assert_eq!(app.yield_unit, YieldUnit::Reads);

        app.toggle_yield_unit();
        assert_eq!(app.yield_unit, YieldUnit::Bases);

        app.toggle_yield_unit();
        assert_eq!(app.yield_unit, YieldUnit::Reads);
    }

    #[test]
    fn test_toggle_outliers() {
        let mut app = App::new(test_config());
        assert!(app.exclude_outliers);

        app.toggle_outliers();
        assert!(!app.exclude_outliers);

        app.toggle_outliers();
        assert!(app.exclude_outliers);
    }

    #[test]
    fn test_update_yield_history() {
        let mut app = App::new(test_config());
        let data = vec![
            YieldDataPoint {
                seconds: 0,
                reads: 0,
                bases: 0,
                reads_passed: 0,
                reads_failed: 0,
                bases_passed: 0,
                bases_failed: 0,
            },
            YieldDataPoint {
                seconds: 60,
                reads: 1000,
                bases: 5_000_000,
                reads_passed: 900,
                reads_failed: 100,
                bases_passed: 4_500_000,
                bases_failed: 500_000,
            },
        ];
        app.update_yield_history("X1", data.clone());
        assert_eq!(app.yield_history.get("X1").unwrap().len(), 2);
    }

    #[test]
    fn test_update_histogram() {
        let mut app = App::new(test_config());
        let histogram = ReadLengthHistogram {
            bucket_ranges: vec![(0, 1000), (1000, 2000)],
            bucket_values: vec![100, 50],
            n50: 1500.0,
            outliers_excluded: false,
            outlier_percent: 0.0,
            requested_range: None,
            source_data_end: 2000,
        };
        app.update_histogram("X1", histogram);
        assert!(app.histograms.contains_key("X1"));
    }

    #[test]
    fn test_update_duty_time() {
        let mut app = App::new(test_config());
        let duty_time = DutyTimeSnapshot {
            time_range: (0, 60),
            state_times: std::collections::HashMap::new(),
            pore_occupancy: vec![0.5, 0.8, 0.3],
        };
        app.update_duty_time("X1", duty_time);
        assert!(app.duty_time.contains_key("X1"));
        assert_eq!(app.duty_time.get("X1").unwrap().pore_occupancy.len(), 3);
    }

    #[test]
    fn test_scroll_offset_initializes_to_zero() {
        let app = App::new(test_config());
        assert_eq!(app.channel_map_scroll_offset, 0);
    }

    #[test]
    fn test_scroll_offset_clamps_to_bounds() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 100;
        app.clamp_channel_map_scroll(28, 20); // total=28, visible=20
        assert_eq!(app.channel_map_scroll_offset, 8); // max = 28-20
    }

    #[test]
    fn test_scroll_reset() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 10;
        app.reset_channel_map_scroll();
        assert_eq!(app.channel_map_scroll_offset, 0);
    }

    #[test]
    fn test_scroll_clamp_handles_zero() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 5;
        app.clamp_channel_map_scroll(20, 20); // total=visible, max_offset=0
        assert_eq!(app.channel_map_scroll_offset, 0);
    }

    #[test]
    fn test_should_scroll_channel_map_in_pore_activity() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A")];
        app.run_states.insert("A".to_string(), RunState::Running);
        app.screen = Screen::PositionDetail { position_idx: 0 };
        app.detail_chart = DetailChart::PoreActivity;
        assert!(app.should_scroll_channel_map());
    }

    #[test]
    fn test_should_not_scroll_channel_map_in_yield() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A")];
        app.run_states.insert("A".to_string(), RunState::Running);
        app.screen = Screen::PositionDetail { position_idx: 0 };
        app.detail_chart = DetailChart::Yield;
        assert!(!app.should_scroll_channel_map());
    }

    #[test]
    fn test_should_not_scroll_channel_map_in_overview() {
        let mut app = App::new(test_config());
        app.detail_chart = DetailChart::PoreActivity;
        app.screen = Screen::Overview;
        assert!(!app.should_scroll_channel_map());
    }

    #[test]
    fn test_scroll_down_increments_offset() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 0;
        let total_rows: usize = 53;
        let visible_rows: usize = 20;
        let old_offset = app.channel_map_scroll_offset;
        app.channel_map_scroll_offset =
            (old_offset + 1).min(total_rows.saturating_sub(visible_rows));
        assert_eq!(app.channel_map_scroll_offset, 1);
    }

    #[test]
    fn test_scroll_down_does_not_exceed_max() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 33; // max = 53-20 = 33
        let total_rows: usize = 53;
        let visible_rows: usize = 20;
        let old_offset = app.channel_map_scroll_offset;
        app.channel_map_scroll_offset =
            (old_offset + 1).min(total_rows.saturating_sub(visible_rows));
        assert_eq!(app.channel_map_scroll_offset, 33); // unchanged
    }

    #[test]
    fn test_scroll_up_decrements_offset() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 5;
        app.channel_map_scroll_offset = app.channel_map_scroll_offset.saturating_sub(1);
        assert_eq!(app.channel_map_scroll_offset, 4);
    }

    #[test]
    fn test_scroll_up_does_not_go_below_zero() {
        let mut app = App::new(test_config());
        app.channel_map_scroll_offset = 0;
        app.channel_map_scroll_offset = app.channel_map_scroll_offset.saturating_sub(1);
        assert_eq!(app.channel_map_scroll_offset, 0); // unchanged
    }
}
