//! Event handling for the TUI.

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        let event_tx = tx.clone();
        std::thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                match event::read() {
                    Ok(CrosstermEvent::Key(key)) => {
                        if event_tx.send(Event::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(CrosstermEvent::Resize(w, h)) => {
                        if event_tx.send(Event::Resize(w, h)).is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            } else if event_tx.send(Event::Tick).is_err() {
                break;
            }
        });

        Self { rx, _tx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Up,
    Down,
    Enter,
    Back,
    Help,
    Refresh,
    Pause,
    Resume,
    Stop,
    ToggleYieldUnit,
    ToggleOutliers,
    ChartYield,
    ChartReadLength,
    ChartPoreActivity,
    CycleChart,
    HistogramSetRange,
    HistogramResetRange,
    ThemeSelector,
    None,
}

impl From<KeyEvent> for Action {
    fn from(key: KeyEvent) -> Self {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Up | KeyCode::Char('k') => Action::Up,
            KeyCode::Down | KeyCode::Char('j') => Action::Down,
            KeyCode::Enter => Action::Enter,
            KeyCode::Esc => Action::Back,
            KeyCode::Char('?') => Action::Help,
            KeyCode::Char('R') => Action::Refresh,
            KeyCode::Char('p') => Action::Pause,
            KeyCode::Char('r') => Action::Resume,
            KeyCode::Char('s') => Action::Stop,
            KeyCode::Char('t') => Action::ToggleYieldUnit,
            KeyCode::Char('o') => Action::ToggleOutliers,
            KeyCode::Char('1') => Action::ChartYield,
            KeyCode::Char('2') => Action::ChartReadLength,
            KeyCode::Char('3') => Action::ChartPoreActivity,
            KeyCode::Tab => Action::CycleChart,
            KeyCode::Char('z') => Action::HistogramSetRange,
            KeyCode::Char('0') => Action::HistogramResetRange,
            KeyCode::Char('T') => Action::ThemeSelector,
            _ => Action::None,
        }
    }
}
