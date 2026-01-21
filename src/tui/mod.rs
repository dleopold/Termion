//! Terminal User Interface.
//!
//! This module implements the TUI using ratatui and crossterm.
//! It provides real-time visualization of sequencing data.

mod app;
mod event;
pub mod theme;
mod ui;

pub use app::{
    App, ChartBuffer, ConnectionState, DetailChart, Overlay, RunControlAction, Screen, YieldUnit,
};
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

    let client_result = Client::connect_with_timeouts(
        &config.connection.host,
        config.connection.port,
        config.connection.connect_timeout,
        config.connection.request_timeout,
    )
    .await;

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
                    use crossterm::event::KeyCode;

                    if matches!(app.overlay, Overlay::RangeInput { .. }) {
                        match key.code {
                            KeyCode::Esc => {
                                app.overlay = Overlay::None;
                            }
                            KeyCode::Enter => {
                                if app.apply_range_input() {
                                    if let Some(ref mut c) = client {
                                        trigger_histogram_refresh(&mut app, c).await;
                                    }
                                }
                            }
                            other => {
                                app.handle_range_input_key(other);
                            }
                        }
                    } else if matches!(app.overlay, Overlay::ThemeSelector { .. }) {
                        match key.code {
                            KeyCode::Esc => {
                                app.overlay = Overlay::None;
                            }
                            KeyCode::Enter => {
                                app.apply_selected_theme();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.theme_selector_up();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.theme_selector_down();
                            }
                            _ => {}
                        }
                    } else if let Some((action, position_name)) = app.pending_confirmation() {
                        match key.code {
                            KeyCode::Esc => {
                                app.clear_confirmation();
                            }
                            KeyCode::Enter => {
                                app.clear_confirmation();
                                execute_run_control(&mut app, &mut client, action, &position_name)
                                    .await;
                            }
                            _ => {}
                        }
                    } else {
                        let action = Action::from(key);
                        handle_action(&mut app, action, &mut client, &config).await;

                        if matches!(action, Action::Refresh) && client.is_none() {
                            reconnect_attempt = 0;
                        }
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

    match Client::connect_with_timeouts(
        &config.connection.host,
        config.connection.port,
        config.connection.connect_timeout,
        config.connection.request_timeout,
    )
    .await
    {
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
    if action != Action::None {
        tracing::debug!(?action, "Handling action");
    }
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
                match Client::connect_with_timeouts(
                    &config.connection.host,
                    config.connection.port,
                    config.connection.connect_timeout,
                    config.connection.request_timeout,
                )
                .await
                {
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
        Action::Pause => {
            app.request_run_control(RunControlAction::Pause);
        }
        Action::Resume => {
            app.request_run_control(RunControlAction::Resume);
        }
        Action::Stop => {
            app.request_run_control(RunControlAction::Stop);
        }
        Action::ToggleYieldUnit => app.toggle_yield_unit(),
        Action::ToggleOutliers => {
            app.toggle_outliers();
            if let Some(ref mut c) = client {
                trigger_histogram_refresh(app, c).await;
            }
        }
        Action::ChartYield => app.set_detail_chart(DetailChart::Yield),
        Action::ChartReadLength => app.set_detail_chart(DetailChart::ReadLength),
        Action::ChartPoreActivity => app.set_detail_chart(DetailChart::PoreActivity),
        Action::CycleChart => app.cycle_detail_chart(),
        Action::HistogramSetRange => app.open_range_input(),
        Action::HistogramResetRange => {
            app.clear_histogram_range();
            if let Some(ref mut c) = client {
                trigger_histogram_refresh(app, c).await;
            }
        }
        Action::ThemeSelector => app.open_theme_selector(),
        Action::None => {}
    }
}

async fn refresh_data(app: &mut App, client: &mut Client) {
    if !app.is_connected() {
        return;
    }

    match client.list_positions().await {
        Ok(positions) => {
            let in_detail_view = matches!(app.screen, Screen::PositionDetail { .. });
            let detail_position_idx = match app.screen {
                Screen::PositionDetail { position_idx } => Some(position_idx),
                _ => None,
            };

            for (idx, pos) in positions.iter().enumerate() {
                if let Ok(mut pos_client) = client.connect_position(pos.clone()).await {
                    if let Ok(stats) = pos_client.get_stats().await {
                        app.update_stats(&pos.name, stats);
                    }

                    if let Ok(run_state) = pos_client.get_run_state().await {
                        app.update_run_state(&pos.name, run_state);
                    }

                    if in_detail_view && detail_position_idx == Some(idx) {
                        fetch_detail_data(app, &mut pos_client).await;
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

async fn fetch_detail_data(app: &mut App, pos_client: &mut crate::client::PositionClient) {
    let position_name = pos_client.position.name.clone();
    tracing::info!(position = %position_name, "Fetching detail data");

    // Check if run is active - streaming endpoints hang for completed runs
    let run_state = app.run_states.get(&position_name).cloned();
    let run_is_active = run_state.as_ref().map(|s| s.is_active()).unwrap_or(false);

    let run_id = match pos_client.get_current_run_id().await {
        Ok(Some(id)) => {
            tracing::debug!(position = %position_name, run_id = %id, active = run_is_active, "Found run");
            id
        }
        Ok(None) => {
            tracing::debug!(position = %position_name, "No run, skipping detail data");
            return;
        }
        Err(e) => {
            tracing::debug!(position = %position_name, error = %e.display_message(), "Failed to get run_id");
            return;
        }
    };

    match pos_client.get_yield_history(&run_id).await {
        Ok(points) if !points.is_empty() => {
            tracing::debug!(position = %position_name, points = points.len(), "Got yield history");

            if let Some(stats) = app.stats_cache.get_mut(&position_name) {
                if points.len() >= 2 {
                    let recent = &points[points.len() - 1];
                    let prev = &points[points.len() - 2];
                    let time_delta = (recent.seconds - prev.seconds).max(1) as f64;
                    let bases_delta = recent.bases.saturating_sub(prev.bases) as f64;
                    stats.throughput_bps = bases_delta / time_delta;
                    stats.throughput_gbph = stats.throughput_bps * 3600.0 / 1_000_000_000.0;
                }
            }

            app.update_yield_history(&position_name, points);
        }
        Ok(_) => {
            tracing::debug!(position = %position_name, "No yield data available");
        }
        Err(e) => {
            tracing::debug!(position = %position_name, error = %e.display_message(), "Yield history failed");
        }
    }

    use futures::StreamExt;
    use std::time::Duration;

    tracing::info!(
        position = %position_name,
        exclude_outliers = app.exclude_outliers,
        range = ?app.histogram_range,
        "Fetching histogram"
    );

    match pos_client
        .stream_read_length_histogram(&run_id, app.exclude_outliers, app.histogram_range)
        .await
    {
        Ok(mut stream) => match tokio::time::timeout(Duration::from_secs(5), stream.next()).await {
            Ok(Some(Ok(histogram))) => {
                tracing::info!(
                    position = %position_name,
                    buckets = histogram.bucket_values.len(),
                    requested_range = ?histogram.requested_range,
                    source_data_end = histogram.source_data_end,
                    "Got histogram"
                );
                app.update_histogram(&position_name, histogram);
            }
            Ok(Some(Err(e))) => {
                tracing::debug!(position = %position_name, error = %e.display_message(), "Histogram stream error");
            }
            Ok(None) => {
                tracing::debug!(position = %position_name, "Histogram stream ended");
            }
            Err(_) => {
                tracing::debug!(position = %position_name, "Histogram fetch timed out");
            }
        },
        Err(e) => {
            tracing::debug!(position = %position_name, error = %e.display_message(), "Histogram stream failed");
        }
    }

    let channel_count = if let Some(layout) = app.channel_layouts.get(&position_name) {
        layout.channel_count as u32
    } else {
        match pos_client.get_channel_layout().await {
            Ok(layout) => {
                tracing::info!(
                    position = %position_name,
                    width = layout.width,
                    height = layout.height,
                    channels = layout.channel_count,
                    "Got channel layout"
                );
                let count = layout.channel_count as u32;
                app.update_channel_layout(&position_name, layout);
                count
            }
            Err(e) => {
                tracing::debug!(position = %position_name, error = %e.display_message(), "Channel layout failed");
                512 // Default fallback for MinION
            }
        }
    };

    if run_is_active {
        match tokio::time::timeout(Duration::from_secs(5), async {
            let mut stream = pos_client.stream_duty_time(&run_id).await?;
            stream.next().await.transpose()
        })
        .await
        {
            Ok(Ok(Some(duty_time))) => {
                let counts = duty_time.pore_counts();
                tracing::debug!(
                    position = %position_name,
                    total_pores = duty_time.pore_occupancy.len(),
                    sequencing = counts.sequencing,
                    pore_available = counts.pore_available,
                    inactive = counts.inactive,
                    unavailable = counts.unavailable,
                    avg_occupancy = %format!("{:.2}", duty_time.average_occupancy()),
                    "Got duty time"
                );

                if !duty_time.pore_occupancy.is_empty() {
                    let sample: Vec<f32> =
                        duty_time.pore_occupancy.iter().take(10).copied().collect();
                    tracing::debug!(sample = ?sample, "First 10 occupancy values");
                }

                if let Some(stats) = app.stats_cache.get_mut(&position_name) {
                    stats.active_pores = duty_time.active_pores(0.1) as u32;
                }

                app.update_duty_time(&position_name, duty_time);
            }
            Ok(Ok(None)) => {
                tracing::debug!(position = %position_name, "No duty time data available");
            }
            Ok(Err(e)) => {
                tracing::debug!(position = %position_name, error = %e.display_message(), "Duty time stream failed");
            }
            Err(_) => {
                tracing::debug!(position = %position_name, "Duty time fetch timed out");
            }
        }
        match tokio::time::timeout(Duration::from_secs(5), pos_client.get_mean_quality(&run_id))
            .await
        {
            Ok(Ok(Some(quality))) => {
                tracing::debug!(position = %position_name, quality = quality, "Got mean quality");
                if let Some(stats) = app.stats_cache.get_mut(&position_name) {
                    stats.mean_quality = quality as f64;
                }
            }
            Ok(Ok(None)) => {
                tracing::debug!(position = %position_name, "No quality data available");
            }
            Ok(Err(e)) => {
                tracing::debug!(position = %position_name, error = %e.display_message(), "Quality boxplot failed");
            }
            Err(_) => {
                tracing::debug!(position = %position_name, "Quality fetch timed out");
            }
        }

        match tokio::time::timeout(
            Duration::from_secs(5),
            pos_client.get_channel_states(channel_count),
        )
        .await
        {
            Ok(Ok(channel_states)) => {
                if let Some(stats) = app.stats_cache.get_mut(&position_name) {
                    stats.active_pores = channel_states.sequencing_count() as u32;
                }
                app.update_channel_states(&position_name, channel_states);
            }
            Ok(Err(e)) => {
                tracing::debug!(position = %position_name, error = %e.display_message(), "Channel states failed");
            }
            Err(_) => {
                tracing::debug!(position = %position_name, "Channel states fetch timed out");
            }
        }
    } else {
        tracing::debug!(position = %position_name, "Skipping streaming endpoints for inactive run");
    }
}

async fn execute_run_control(
    app: &mut App,
    client: &mut Option<Client>,
    action: RunControlAction,
    position_name: &str,
) {
    let Some(ref mut c) = client else {
        app.set_error("Not connected to MinKNOW".to_string());
        return;
    };

    let position = match app.positions.iter().find(|p| p.name == position_name) {
        Some(p) => p.clone(),
        None => {
            app.set_error(format!("Position {} not found", position_name));
            return;
        }
    };

    let mut pos_client = match c.connect_position(position).await {
        Ok(pc) => pc,
        Err(e) => {
            app.set_error(format!(
                "Failed to connect to position: {}",
                e.display_message()
            ));
            return;
        }
    };

    let result = match action {
        RunControlAction::Pause => pos_client.pause().await,
        RunControlAction::Resume => pos_client.resume().await,
        RunControlAction::Stop => pos_client.stop_protocol().await,
    };

    if let Err(e) = result {
        app.set_error(format!(
            "Failed to {} run: {}",
            action.label().to_lowercase(),
            e.display_message()
        ));
    } else {
        tracing::info!(
            action = action.label(),
            position = position_name,
            "Run control action executed"
        );
    }
}

async fn trigger_histogram_refresh(app: &mut App, client: &mut Client) {
    let position = match app.selected_position() {
        Some(p) => p.clone(),
        None => return,
    };

    let mut pos_client = match client.connect_position(position.clone()).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e.display_message(), "Failed to connect for histogram refresh");
            return;
        }
    };

    let run_id = match pos_client.get_current_run_id().await {
        Ok(Some(id)) => id,
        Ok(None) => return,
        Err(_) => return,
    };

    tracing::info!(
        position = %position.name,
        exclude_outliers = app.exclude_outliers,
        range = ?app.histogram_range,
        "Immediate histogram refresh"
    );

    match pos_client
        .stream_read_length_histogram(&run_id, app.exclude_outliers, app.histogram_range)
        .await
    {
        Ok(mut stream) => {
            use futures::StreamExt;
            if let Some(Ok(histogram)) = stream.next().await {
                tracing::info!(
                    position = %position.name,
                    buckets = histogram.bucket_values.len(),
                    "Got histogram (immediate)"
                );
                app.update_histogram(&position.name, histogram);
            }
        }
        Err(e) => {
            tracing::debug!(error = %e.display_message(), "Histogram refresh failed");
        }
    }
}
