//! UI rendering functions.

use super::app::{App, ConnectionState, DetailChart, Overlay, Screen, YieldUnit};
use crate::client::{
    ChannelLayout, ChannelStatesSnapshot, Position, PositionState, ReadLengthHistogram,
    StatsSnapshot,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row,
        Table, Wrap,
    },
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

fn render_footer(frame: &mut Frame, _app: &App, area: Rect) {
    let hints = "[↑↓] Navigate  [Enter] Details  [q] Quit  [?] Help";

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

fn render_detail_footer(frame: &mut Frame, app: &App, area: Rect) {
    let chart_hints = match app.detail_chart {
        DetailChart::Yield => {
            let unit = match app.yield_unit {
                YieldUnit::Bases => "bases",
                YieldUnit::Reads => "reads",
            };
            format!("[b] Toggle ({})  ", unit)
        }
        DetailChart::ReadLength => {
            let outliers = if app.exclude_outliers {
                "excluding"
            } else {
                "including"
            };
            format!("[o] Outliers ({})  ", outliers)
        }
        DetailChart::PoreActivity => String::new(),
    };

    let hints = format!("[Esc] Back  [1/2/3|Tab] Charts  {}[?] Help", chart_hints);

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
            Constraint::Length(3),
        ])
        .split(area);

    render_detail_header(frame, position, chunks[0]);
    render_run_info(frame, position, stats, chunks[1]);

    match app.detail_chart {
        DetailChart::Yield => render_yield_chart(frame, app, &position.name, chunks[2]),
        DetailChart::ReadLength => {
            let histogram = app.histograms.get(&position.name);
            render_histogram_chart(frame, histogram, app.exclude_outliers, chunks[2]);
        }
        DetailChart::PoreActivity => {
            let channel_states = app.channel_states.get(&position.name);
            let channel_layout = app.channel_layouts.get(&position.name);
            render_pore_activity(frame, channel_states, channel_layout, chunks[2]);
        }
    }

    render_detail_footer(frame, app, chunks[3]);
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
    let content = if let Some(s) = stats {
        vec![
            Line::from(vec![
                Span::styled("Reads: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_number(s.reads_processed),
                    Style::default().bold().fg(Color::Cyan),
                ),
                Span::raw("  "),
                Span::styled(
                    format_number(s.reads_passed),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(" passed  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_number(s.reads_failed),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(" failed", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("Bases: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_bytes(s.bases_called),
                    Style::default().bold().fg(Color::Cyan),
                ),
                Span::raw("  "),
                Span::styled(
                    format_bytes(s.bases_passed),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(" passed  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_bytes(s.bases_failed),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(" failed", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::styled("Throughput: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.2} Gb/h", s.throughput_gbph),
                    Style::default().bold(),
                ),
                Span::raw("    "),
                Span::styled("Pass Rate: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:.1}%", s.pass_rate()), Style::default().bold()),
                Span::raw("    "),
                Span::styled("Active Pores: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format_number(s.active_pores as u64),
                    Style::default().bold(),
                ),
                Span::raw("    "),
                Span::styled("Q: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if s.mean_quality > 0.0 {
                        format!("{:.1}", s.mean_quality)
                    } else {
                        "--".to_string()
                    },
                    Style::default().bold(),
                ),
                Span::raw("    "),
                Span::styled("Len: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if s.mean_read_length > 0.0 {
                        format_number(s.mean_read_length as u64)
                    } else {
                        "--".to_string()
                    },
                    Style::default().bold(),
                ),
            ]),
        ]
    } else {
        vec![Line::from("No data available")]
    };

    let info = Paragraph::new(content).block(
        Block::default()
            .title(" Run Info ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(info, area);
}

fn render_yield_chart(frame: &mut Frame, app: &App, position_name: &str, area: Rect) {
    let yield_data = app.yield_history.get(position_name);

    let yield_points = match yield_data {
        Some(points) if !points.is_empty() => points,
        _ => {
            let placeholder = Paragraph::new("Waiting for data...")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Cumulative Yield [1] ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                );
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let min_x = yield_points
        .first()
        .map(|p| p.seconds as f64)
        .unwrap_or(0.0);
    let max_x = yield_points.last().map(|p| p.seconds as f64).unwrap_or(1.0);

    type ValueFn = fn(&crate::client::YieldDataPoint) -> u64;
    let (get_total, get_passed, get_failed): (ValueFn, ValueFn, ValueFn) = match app.yield_unit {
        YieldUnit::Bases => (|p| p.bases, |p| p.bases_passed, |p| p.bases_failed),
        YieldUnit::Reads => (|p| p.reads, |p| p.reads_passed, |p| p.reads_failed),
    };

    let max_raw_value = yield_points
        .iter()
        .map(|p| get_total(p).max(get_passed(p)).max(get_failed(p)))
        .max()
        .unwrap_or(0) as f64;

    let (title, scale_factor): (&str, f64) = match app.yield_unit {
        YieldUnit::Bases => {
            if max_raw_value >= 1_000_000_000_000.0 {
                ("Cumulative Yield (Tb)", 1_000_000_000_000.0)
            } else if max_raw_value >= 1_000_000_000.0 {
                ("Cumulative Yield (Gb)", 1_000_000_000.0)
            } else if max_raw_value >= 1_000_000.0 {
                ("Cumulative Yield (Mb)", 1_000_000.0)
            } else if max_raw_value >= 1_000.0 {
                ("Cumulative Yield (Kb)", 1_000.0)
            } else {
                ("Cumulative Yield (b)", 1.0)
            }
        }
        YieldUnit::Reads => {
            if max_raw_value >= 1_000_000_000.0 {
                ("Cumulative Yield (B reads)", 1_000_000_000.0)
            } else if max_raw_value >= 1_000_000.0 {
                ("Cumulative Yield (M reads)", 1_000_000.0)
            } else if max_raw_value >= 1_000.0 {
                ("Cumulative Yield (K reads)", 1_000.0)
            } else {
                ("Cumulative Yield (reads)", 1.0)
            }
        }
    };

    let total_data: Vec<(f64, f64)> = yield_points
        .iter()
        .map(|p| (p.seconds as f64 - min_x, get_total(p) as f64 / scale_factor))
        .collect();
    let passed_data: Vec<(f64, f64)> = yield_points
        .iter()
        .map(|p| {
            (
                p.seconds as f64 - min_x,
                get_passed(p) as f64 / scale_factor,
            )
        })
        .collect();
    let failed_data: Vec<(f64, f64)> = yield_points
        .iter()
        .map(|p| {
            (
                p.seconds as f64 - min_x,
                get_failed(p) as f64 / scale_factor,
            )
        })
        .collect();

    let all_y_values = total_data
        .iter()
        .chain(passed_data.iter())
        .chain(failed_data.iter())
        .map(|(_, y)| *y);

    let data_min_y = all_y_values.clone().fold(f64::INFINITY, f64::min);
    let data_max_y = all_y_values.fold(0.0f64, f64::max);

    let y_range = data_max_y - data_min_y;
    let y_padding = if y_range > 0.001 {
        y_range * 0.1
    } else {
        data_max_y * 0.1 + 0.001
    };

    let min_y = (data_min_y - y_padding).max(0.0);
    let max_y = data_max_y + y_padding;

    // Order matters: later datasets render on top. We want total > passed > failed.
    let datasets = vec![
        Dataset::default()
            .name("Failed")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red))
            .data(&failed_data),
        Dataset::default()
            .name("Passed")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Green))
            .data(&passed_data),
        Dataset::default()
            .name("Total")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&total_data),
    ];

    let time_label = format_time_label(max_x - min_x);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!(" {} [1] ", title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, max_x - min_x])
                .labels(vec![Line::from("0"), Line::from(time_label)]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([min_y, max_y])
                .labels(vec![
                    Line::from(format!("{:.1}", min_y)),
                    Line::from(format!("{:.1}", (min_y + max_y) / 2.0)),
                    Line::from(format!("{:.1}", max_y)),
                ]),
        )
        .legend_position(None);

    frame.render_widget(chart, area);
}

fn render_histogram_chart(
    frame: &mut Frame,
    histogram: Option<&ReadLengthHistogram>,
    exclude_outliers: bool,
    area: Rect,
) {
    let title = if exclude_outliers {
        " Read Length Distribution (outliers excluded) [2] "
    } else {
        " Read Length Distribution [2] "
    };

    let histogram = match histogram {
        Some(h) if !h.bucket_values.is_empty() => h,
        _ => {
            let placeholder = Paragraph::new("Waiting for histogram data...")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Magenta)),
                );
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let max_count = histogram.max_value();
    let num_buckets = histogram.bucket_values.len().min(20);

    let bars: Vec<Bar> = histogram
        .bucket_ranges
        .iter()
        .zip(histogram.bucket_values.iter())
        .take(num_buckets)
        .map(|((start, end), &count)| {
            let label = if *end >= 10000 {
                format!("{}k", start / 1000)
            } else {
                format!("{}", start)
            };
            Bar::default()
                .value(count)
                .label(Line::from(label))
                .style(Style::default().fg(Color::Magenta))
        })
        .collect();

    let n50_info = format!("N50: {} bp", format_number(histogram.n50 as u64));

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(title)
                .title_bottom(Line::from(n50_info).alignment(Alignment::Right))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(3)
        .bar_gap(1)
        .max(max_count);

    frame.render_widget(bar_chart, area);
}

fn render_pore_activity(
    frame: &mut Frame,
    channel_states: Option<&ChannelStatesSnapshot>,
    channel_layout: Option<&ChannelLayout>,
    area: Rect,
) {
    let title = " Pore Activity [3] ";

    let channel_states = match channel_states {
        Some(cs) if !cs.states.is_empty() => cs,
        _ => {
            let placeholder = Paragraph::new("Waiting for channel data...")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                );
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_pore_grid_from_states(frame, channel_states, channel_layout, chunks[0]);
    render_state_counts(frame, channel_states, chunks[1]);
}

fn render_pore_grid_from_states(
    frame: &mut Frame,
    channel_states: &ChannelStatesSnapshot,
    channel_layout: Option<&ChannelLayout>,
    area: Rect,
) {
    let inner = Block::default()
        .title(" Channel Map ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner_area = inner.inner(area);
    frame.render_widget(inner, area);

    let screen_width = inner_area.width as usize;
    let screen_height = inner_area.height as usize;

    if screen_width == 0 || screen_height == 0 || channel_states.states.is_empty() {
        return;
    }

    let total_channels = channel_states.states.len();

    let (grid_width, grid_height) = if let Some(layout) = channel_layout {
        (layout.width as usize, layout.height as usize)
    } else {
        let size = (total_channels as f64).sqrt().ceil() as usize;
        (size, size)
    };

    let has_two_blocks = grid_height == 16;
    let gap_after_row = if has_two_blocks { Some(7usize) } else { None };
    let total_display_height = if has_two_blocks {
        grid_height + 1
    } else {
        grid_height
    };

    let cell_char_width = 2usize;
    let scale_x = ((grid_width * cell_char_width) as f64 / screen_width as f64).max(1.0);
    let scale_y = (total_display_height as f64 / screen_height as f64).max(1.0);
    let scale = scale_x.max(scale_y);

    let display_cols =
        ((grid_width as f64 / scale).ceil() as usize).min(screen_width / cell_char_width);
    let display_rows = ((total_display_height as f64 / scale).ceil() as usize).min(screen_height);

    let grid_pixel_width = display_cols * cell_char_width;
    let grid_pixel_height = display_rows;
    let offset_x = (screen_width.saturating_sub(grid_pixel_width)) / 2;
    let offset_y = (screen_height.saturating_sub(grid_pixel_height)) / 2;

    let coord_to_channel: std::collections::HashMap<(u32, u32), usize> =
        if let Some(layout) = channel_layout {
            layout
                .coords
                .iter()
                .enumerate()
                .map(|(idx, &coord)| (coord, idx))
                .collect()
        } else {
            std::collections::HashMap::new()
        };

    let mut lines: Vec<Line> = Vec::with_capacity(screen_height);

    for _ in 0..offset_y {
        lines.push(Line::from(""));
    }

    let mut grid_row = 0usize;
    for display_row in 0..display_rows {
        if let Some(gap_row) = gap_after_row {
            let gap_display_row = ((gap_row + 1) as f64 / scale).floor() as usize;
            if display_row == gap_display_row && scale <= 1.5 {
                lines.push(Line::from(""));
                continue;
            }
        }

        let padding = " ".repeat(offset_x);
        let mut spans: Vec<Span> = vec![Span::raw(padding)];

        for display_col in 0..display_cols {
            let grid_x = (display_col as f64 * scale).floor() as u32;
            let grid_y = grid_row as u32;

            let channel_idx = if channel_layout.is_some() {
                coord_to_channel.get(&(grid_x, grid_y)).copied()
            } else {
                let idx = (grid_y as usize) * grid_width + (grid_x as usize);
                if idx < total_channels {
                    Some(idx)
                } else {
                    None
                }
            };

            let (symbol, color) = match channel_idx {
                Some(idx) if idx < channel_states.states.len() => {
                    state_to_symbol(&channel_states.states[idx])
                }
                _ => ("  ", Color::Black),
            };
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
        grid_row = (grid_row + 1).min(grid_height.saturating_sub(1));
    }

    let grid = Paragraph::new(lines);
    frame.render_widget(grid, inner_area);
}

fn render_state_counts(frame: &mut Frame, channel_states: &ChannelStatesSnapshot, area: Rect) {
    let total = channel_states.channel_count;

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled("Channel States", Style::default().bold())),
        Line::from(""),
    ];

    let sequencing = channel_states.sequencing_count();
    let pore_available = channel_states.pore_available_count();
    let unavailable = channel_states.unavailable_count();
    let inactive = channel_states.inactive_count();
    let other = total.saturating_sub(sequencing + pore_available + unavailable + inactive);

    let categories = [
        ("Sequencing", sequencing, Color::Green),
        ("Pore Available", pore_available, Color::Blue),
        ("Unavailable", unavailable, Color::Magenta),
        ("Inactive", inactive, Color::Cyan),
        ("Other", other, Color::DarkGray),
    ];

    for (label, count, color) in &categories {
        let percent = if total > 0 {
            (*count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        lines.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(*color)),
            Span::styled(format!("{:>4}", count), Style::default().bold()),
            Span::styled(format!(" {:14}", label), Style::default()),
            Span::styled(
                format!("{:5.1}%", percent),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", total), Style::default().bold()),
        Span::raw(" channels"),
    ]));

    let breakdown = Paragraph::new(lines).block(
        Block::default()
            .title(" Statistics ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(breakdown, area);
}

fn state_to_symbol(state: &str) -> (&'static str, Color) {
    let s = state.to_lowercase();
    if s.contains("strand") || s.contains("sequencing") {
        ("██", Color::Green)
    } else if s.contains("pore") || s.contains("single") {
        ("██", Color::Blue)
    } else if s.contains("unavailable") || s.contains("saturated") {
        ("░░", Color::Magenta)
    } else if s.contains("inactive") || s.contains("zero") || s.contains("multiple") {
        ("░░", Color::Cyan)
    } else if s.contains("adapter") || s.contains("event") {
        ("▓▓", Color::Yellow)
    } else if s.contains("unblock") {
        ("▒▒", Color::Yellow)
    } else if s.is_empty() || s == "unknown" {
        ("  ", Color::Black)
    } else {
        ("░░", Color::DarkGray)
    }
}

fn format_time_label(seconds: f64) -> String {
    if seconds >= 3600.0 {
        format!("{:.1}h", seconds / 3600.0)
    } else if seconds >= 60.0 {
        format!("{:.0}m", seconds / 60.0)
    } else {
        format!("{:.0}s", seconds)
    }
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
        Line::from(Span::styled("Detail View", Style::default().bold())),
        Line::from("  1          Yield chart"),
        Line::from("  2          Read length histogram"),
        Line::from("  3          Pore activity"),
        Line::from("  Tab        Cycle charts"),
        Line::from("  b          Toggle bases/reads"),
        Line::from("  o          Toggle outlier exclusion"),
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
