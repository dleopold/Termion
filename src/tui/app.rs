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
