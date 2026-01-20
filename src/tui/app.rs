//! Application state and core logic.

use crate::client::{Position, StatsSnapshot};
use crate::config::Config;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Overview,
    PositionDetail { position_idx: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    None,
    Help,
    Error { message: String },
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
    pub screen: Screen,
    pub overlay: Overlay,
    pub connection: ConnectionState,
    pub positions: Vec<Position>,
    pub selected_position: usize,
    pub stats_cache: HashMap<String, StatsSnapshot>,
    pub chart_data: HashMap<String, ChartBuffer>,
    pub should_quit: bool,
    pub last_error: Option<String>,
}

pub struct ChartBuffer {
    pub data: Vec<(f64, f64)>,
    pub max_points: usize,
}

impl ChartBuffer {
    pub fn new(max_points: usize) -> Self {
        Self {
            data: Vec::with_capacity(max_points),
            max_points,
        }
    }

    pub fn push(&mut self, timestamp: f64, value: f64) {
        if self.data.len() >= self.max_points {
            self.data.remove(0);
        }
        self.data.push((timestamp, value));
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            screen: Screen::Overview,
            overlay: Overlay::None,
            connection: ConnectionState::Connecting,
            positions: Vec::new(),
            selected_position: 0,
            stats_cache: HashMap::new(),
            chart_data: HashMap::new(),
            should_quit: false,
            last_error: None,
        }
    }

    pub fn select_next(&mut self) {
        if !self.positions.is_empty() {
            self.selected_position = (self.selected_position + 1) % self.positions.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.positions.is_empty() {
            self.selected_position = self
                .selected_position
                .checked_sub(1)
                .unwrap_or(self.positions.len() - 1);
        }
    }

    pub fn enter_detail(&mut self) {
        if !self.positions.is_empty() {
            self.screen = Screen::PositionDetail {
                position_idx: self.selected_position,
            };
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

    pub fn update_stats(&mut self, position_name: &str, stats: StatsSnapshot) {
        let chart = self
            .chart_data
            .entry(position_name.to_string())
            .or_insert_with(|| ChartBuffer::new(1800));

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        chart.push(now, stats.throughput_gbph);
        self.stats_cache.insert(position_name.to_string(), stats);
    }

    pub fn selected_position(&self) -> Option<&Position> {
        self.positions.get(self.selected_position)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionState::Connected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::PositionState;

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
    fn test_enter_detail_with_positions() {
        let mut app = App::new(test_config());
        app.positions = vec![test_position("A")];
        app.enter_detail();
        assert_eq!(app.screen, Screen::PositionDetail { position_idx: 0 });
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
}
