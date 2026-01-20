//! Terminal User Interface.
//!
//! This module implements the TUI using ratatui and crossterm.
//! It provides real-time visualization of sequencing data.

mod app;
mod event;
mod ui;

pub use app::{App, ChartBuffer, ConnectionState, Overlay, Screen};
pub use event::{Action, Event, EventHandler};

use crate::client::Client;
use crate::config::Config;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub async fn run(config: Config) -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, config).await;
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
) -> anyhow::Result<()> {
    let mut app = App::new(config.clone());
    let tick_rate = config.tui.refresh_interval;
    let mut events = EventHandler::new(tick_rate);
    let mut reconnect_attempt = 0u32;
    let mut last_reconnect = std::time::Instant::now();

    let client_result = Client::connect(&config.connection.host, config.connection.port).await;

    let mut client = match client_result {
        Ok(c) => {
            app.set_connected();
            Some(c)
        }
        Err(e) => {
            app.set_disconnected(e.display_message());
            None
        }
    };

    if let Some(ref mut c) = client {
        match c.list_positions().await {
            Ok(positions) => app.update_positions(positions),
            Err(e) => app.set_error(e.display_message()),
        }
    }

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(event) = events.next().await {
            match event {
                Event::Key(key) => {
                    let action = Action::from(key);
                    handle_action(&mut app, action, &mut client, &config).await;

                    if matches!(action, Action::Refresh) && client.is_none() {
                        reconnect_attempt = 0;
                    }
                }
                Event::Tick => {
                    if let Some(ref mut c) = client {
                        refresh_data(&mut app, c).await;

                        if !app.is_connected() {
                            client = None;
                            reconnect_attempt = 0;
                        }
                    } else {
                        try_reconnect(
                            &mut app,
                            &mut client,
                            &config,
                            &mut reconnect_attempt,
                            &mut last_reconnect,
                        )
                        .await;
                    }
                }
                Event::Resize(_, _) => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn try_reconnect(
    app: &mut App,
    client: &mut Option<Client>,
    config: &Config,
    attempt: &mut u32,
    last_attempt: &mut std::time::Instant,
) {
    let backoff = std::time::Duration::from_millis(
        (config.reconnect.initial_delay.as_millis() as f64
            * config.reconnect.multiplier.powi(*attempt as i32)) as u64,
    )
    .min(config.reconnect.max_delay);

    if last_attempt.elapsed() < backoff {
        return;
    }

    *attempt += 1;
    *last_attempt = std::time::Instant::now();
    app.set_reconnecting(*attempt);

    match Client::connect(&config.connection.host, config.connection.port).await {
        Ok(c) => {
            *client = Some(c);
            *attempt = 0;
            app.set_connected();

            if let Some(ref mut c) = client {
                if let Ok(positions) = c.list_positions().await {
                    app.update_positions(positions);
                }
            }
        }
        Err(e) => {
            app.set_disconnected(e.display_message());
        }
    }
}

async fn handle_action(
    app: &mut App,
    action: Action,
    client: &mut Option<Client>,
    config: &Config,
) {
    match action {
        Action::Quit => app.quit(),
        Action::Up => app.select_previous(),
        Action::Down => app.select_next(),
        Action::Enter => app.enter_detail(),
        Action::Back => app.back(),
        Action::Help => app.toggle_help(),
        Action::Refresh => {
            if let Some(ref mut c) = client {
                match c.list_positions().await {
                    Ok(positions) => app.update_positions(positions),
                    Err(e) => app.set_error(e.display_message()),
                }
            } else {
                match Client::connect(&config.connection.host, config.connection.port).await {
                    Ok(c) => {
                        *client = Some(c);
                        app.set_connected();
                        if let Some(ref mut c) = client {
                            if let Ok(positions) = c.list_positions().await {
                                app.update_positions(positions);
                            }
                        }
                    }
                    Err(e) => {
                        app.set_disconnected(e.display_message());
                    }
                }
            }
        }
        Action::Pause | Action::Resume | Action::Stop => {
            // TODO: Implement run control actions
        }
        Action::None => {}
    }
}

async fn refresh_data(app: &mut App, client: &mut Client) {
    if !app.is_connected() {
        return;
    }

    match client.list_positions().await {
        Ok(positions) => {
            for pos in &positions {
                if let Ok(mut pos_client) = client.connect_position(pos.clone()).await {
                    if let Ok(stats) = pos_client.get_stats().await {
                        app.update_stats(&pos.name, stats);
                    }
                }
            }
            app.update_positions(positions);
        }
        Err(e) => {
            app.set_disconnected(e.display_message());
        }
    }
}
