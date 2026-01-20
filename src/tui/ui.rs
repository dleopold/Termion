//! UI rendering functions.

use super::app::{App, ConnectionState, Overlay, Screen};
use crate::client::{Position, PositionState, StatsSnapshot};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    match &app.screen {
        Screen::Overview => render_overview(frame, app, area),
        Screen::PositionDetail { position_idx } => {
            render_position_detail(frame, app, *position_idx, area)
        }
    }

    if let Some(overlay_area) = centered_rect(60, 60, area) {
        match &app.overlay {
            Overlay::Help => render_help_overlay(frame, overlay_area),
            Overlay::Error { message } => render_error_overlay(frame, message, overlay_area),
            Overlay::None => {}
        }
    }
}

fn render_overview(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_position_table(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let status = match &app.connection {
        ConnectionState::Connected => {
            Span::styled(" ● Connected ", Style::default().fg(Color::Green))
        }
        ConnectionState::Connecting => {
            Span::styled(" ◌ Connecting... ", Style::default().fg(Color::Yellow))
        }
        ConnectionState::Disconnected { reason, .. } => Span::styled(
            format!(" ○ Disconnected: {} ", reason),
            Style::default().fg(Color::Red),
        ),
        ConnectionState::Reconnecting { attempt } => Span::styled(
            format!(" ◌ Reconnecting (attempt {})... ", attempt),
            Style::default().fg(Color::Yellow),
        ),
    };

    let title = Line::from(vec![
        Span::styled(" Termion ", Style::default().bold()),
        Span::raw("│"),
        status,
    ]);

    let header = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

fn render_position_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        "Position",
        "State",
        "Run ID",
        "Reads",
        "Bases",
        "Trend",
        "Throughput",
    ])
    .style(Style::default().bold())
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .positions
        .iter()
        .enumerate()
        .map(|(idx, pos)| {
            let stats = app.stats_cache.get(&pos.name);

            let state_indicator = match pos.state {
                PositionState::Running => "● Running",
                PositionState::Idle => "○ Idle",
                PositionState::Error => "✖ Error",
                _ => "? Unknown",
            };

            let reads = stats
                .map(|s| format_number(s.reads_processed))
                .unwrap_or_else(|| "--".to_string());

            let bases = stats
                .map(|s| format_bytes(s.bases_called))
                .unwrap_or_else(|| "--".to_string());

            let throughput = stats
                .map(|s| format!("{:.2} Gb/h", s.throughput_gbph))
                .unwrap_or_else(|| "--".to_string());

            let sparkline = app
                .chart_data
                .get(&pos.name)
                .map(|c| render_mini_sparkline(&c.data, 10))
                .unwrap_or_else(|| "░░░░░░░░░░".to_string());

            let run_id = stats
                .map(|_| pos.name.clone())
                .unwrap_or_else(|| "--".to_string());

            let style = if idx == app.selected_position {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                pos.name.clone(),
                state_indicator.to_string(),
                run_id,
                reads,
                bases,
                sparkline,
                throughput,
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(20),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" Positions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hints = match &app.screen {
        Screen::Overview => "[↑↓] Navigate  [Enter] Details  [q] Quit  [?] Help",
        Screen::PositionDetail { .. } => "[Esc] Back  [p] Pause  [r] Resume  [s] Stop  [?] Help",
    };

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

fn render_position_detail(frame: &mut Frame, app: &App, position_idx: usize, area: Rect) {
    let position = match app.positions.get(position_idx) {
        Some(p) => p,
        None => {
            let msg = Paragraph::new("Position not found")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }
    };

    let stats = app.stats_cache.get(&position.name);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(area);

    render_detail_header(frame, position, chunks[0]);
    render_run_info(frame, position, stats, chunks[1]);
    render_throughput_chart(frame, app, &position.name, chunks[2]);
    render_stats_grid(frame, stats, chunks[3]);
    render_footer(frame, app, chunks[4]);
}

fn render_detail_header(frame: &mut Frame, position: &Position, area: Rect) {
    let state_color = match position.state {
        PositionState::Running => Color::Green,
        PositionState::Idle => Color::DarkGray,
        PositionState::Error => Color::Red,
        _ => Color::White,
    };

    let state_text = match position.state {
        PositionState::Running => "Running",
        PositionState::Idle => "Idle",
        PositionState::Error => "Error",
        _ => "Unknown",
    };

    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", position.name),
            Style::default().bold().fg(Color::Cyan),
        ),
        Span::raw("── "),
        Span::styled(state_text, Style::default().fg(state_color).bold()),
    ]);

    let header = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

fn render_run_info(
    frame: &mut Frame,
    _position: &Position,
    stats: Option<&StatsSnapshot>,
    area: Rect,
) {
    let info_text = if let Some(stats) = stats {
        format!(
            "Reads: {}  │  Passed: {}  │  Failed: {}",
            format_number(stats.reads_processed),
            format_number(stats.reads_passed),
            format_number(stats.reads_failed),
        )
    } else {
        "No data available".to_string()
    };

    let info = Paragraph::new(info_text).style(Style::default()).block(
        Block::default()
            .title(" Run Info ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(info, area);
}

fn render_throughput_chart(frame: &mut Frame, app: &App, position_name: &str, area: Rect) {
    let chart_data = app.chart_data.get(position_name);

    let data: Vec<(f64, f64)> = chart_data.map(|c| c.data.clone()).unwrap_or_default();

    if data.is_empty() {
        let placeholder = Paragraph::new("Waiting for data...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Throughput ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(placeholder, area);
        return;
    }

    let min_x = data.first().map(|(x, _)| *x).unwrap_or(0.0);
    let max_x = data.last().map(|(x, _)| *x).unwrap_or(1.0);
    let max_y = data.iter().map(|(_, y)| *y).fold(0.0f64, f64::max).max(0.1);

    let normalized: Vec<(f64, f64)> = data.iter().map(|(x, y)| (x - min_x, *y)).collect();

    let datasets = vec![Dataset::default()
        .name("Gb/h")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&normalized)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(" Throughput ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, max_x - min_x])
                .labels(vec![Line::from(""), Line::from("now")]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, max_y * 1.1])
                .labels(vec![
                    Line::from("0"),
                    Line::from(format!("{:.1}", max_y / 2.0)),
                    Line::from(format!("{:.1}", max_y)),
                ]),
        );

    frame.render_widget(chart, area);
}

fn render_stats_grid(frame: &mut Frame, stats: Option<&StatsSnapshot>, area: Rect) {
    let content = if let Some(s) = stats {
        vec![
            Line::from(vec![
                Span::styled("Reads: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format_number(s.reads_processed), Style::default().bold()),
                Span::raw("    "),
                Span::styled("Bases: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format_bytes(s.bases_called), Style::default().bold()),
            ]),
            Line::from(vec![
                Span::styled("Passed: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_number(s.reads_passed),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("    "),
                Span::styled("Failed: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_number(s.reads_failed),
                    Style::default().fg(Color::Red),
                ),
            ]),
            Line::from(vec![
                Span::styled("Throughput: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.2} Gb/h", s.throughput_gbph),
                    Style::default().bold().fg(Color::Cyan),
                ),
            ]),
        ]
    } else {
        vec![Line::from("No statistics available")]
    };

    let stats_widget = Paragraph::new(content).block(
        Block::default()
            .title(" Statistics ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(stats_widget, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled(
            "Help",
            Style::default().bold().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation", Style::default().bold())),
        Line::from("  ↑/↓        Move selection"),
        Line::from("  Enter      Select / Drill down"),
        Line::from("  Esc        Back / Close"),
        Line::from(""),
        Line::from(Span::styled("Actions", Style::default().bold())),
        Line::from("  p          Pause acquisition"),
        Line::from("  r          Resume acquisition"),
        Line::from("  s          Stop acquisition"),
        Line::from("  R          Force refresh"),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().bold())),
        Line::from("  ?          Toggle help"),
        Line::from("  q          Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "[Esc] Close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .style(Style::default().bg(Color::Black)),
        );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(help, area);
}

fn render_error_overlay(frame: &mut Frame, message: &str, area: Rect) {
    let error = Paragraph::new(vec![
        Line::from(Span::styled(
            "Error",
            Style::default().bold().fg(Color::Red),
        )),
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(Span::styled(
            "[Esc] Close",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .style(Style::default().bg(Color::Black)),
    );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(error, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Option<Rect> {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);

    Some(popup_area[1])
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} Gb", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.2} Mb", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.2} Kb", bytes as f64 / 1_000.0)
    } else {
        format!("{} b", bytes)
    }
}

fn render_mini_sparkline(data: &[(f64, f64)], width: usize) -> String {
    if data.is_empty() {
        return "░".repeat(width);
    }

    let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let values: Vec<f64> = if data.len() <= width {
        data.iter().map(|(_, y)| *y).collect()
    } else {
        let step = data.len() / width;
        (0..width)
            .map(|i| {
                let start = i * step;
                let end = ((i + 1) * step).min(data.len());
                data[start..end].iter().map(|(_, y)| *y).sum::<f64>() / (end - start) as f64
            })
            .collect()
    };

    let max_val = values.iter().fold(0.0f64, |a, &b| a.max(b));
    if max_val == 0.0 {
        return "░".repeat(width);
    }

    let mut result = String::with_capacity(width * 3);
    for &val in &values {
        let normalized = (val / max_val * 7.0).round() as usize;
        let idx = normalized.min(7);
        result.push(bars[idx]);
    }

    while result.chars().count() < width {
        result.insert(0, '░');
    }

    result
}
