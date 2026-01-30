//! UI rendering functions.

use super::app::{App, ConnectionState, DetailChart, Overlay, RunControlAction, Screen, YieldUnit};
use super::theme::Theme;
use crate::client::{
    ChannelLayout, ChannelStatesSnapshot, Position, PositionState, ReadLengthHistogram, RunState,
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

/// Represents the type of flow cell (device) based on channel count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlowCellType {
    #[default]
    MinION,
    PromethION,
}

impl FlowCellType {
    /// Infers the flow cell type from the channel count.
    ///
    /// # Arguments
    ///
    /// * `count` - Total number of channels
    ///
    /// # Returns
    ///
    /// - `MinION` if count <= 512 (includes Flongle with 126 channels)
    /// - `PromethION` if count > 512
    #[allow(dead_code)]
    pub fn from_channel_count(count: usize) -> Self {
        if count > 512 {
            FlowCellType::PromethION
        } else {
            FlowCellType::MinION
        }
    }
}

/// Describes how channels are arranged in blocks within a flow cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum BlockArrangement {
    /// MinION: 2 blocks stacked vertically
    TwoVertical,
    /// PromethION: 2×2 grid of quadrants
    FourQuadrant,
}

/// Specifies the position of a gap in the channel grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum GapPosition {
    /// Gap after specified row (horizontal gap)
    Horizontal { after_row: usize },
    /// Gap after specified column (vertical gap)
    Vertical { after_col: usize },
}

/// Describes the grid structure for rendering a channel map.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct GridStructure {
    /// Type of flow cell (MinION or PromethION)
    pub flow_cell_type: FlowCellType,
    /// Normalized column count for the grid
    pub grid_cols: usize,
    /// Normalized row count for the grid
    pub grid_rows: usize,
    /// How channels are arranged in blocks
    pub block_arrangement: BlockArrangement,
    /// Positions of gaps in the grid
    pub gap_positions: Vec<GapPosition>,
    /// Width of each cell in characters (1 or 2)
    pub cell_width: usize,
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let t = &app.theme;

    match &app.screen {
        Screen::Overview => render_overview(frame, app, area),
        Screen::PositionDetail { position_idx } => {
            render_position_detail(frame, app, *position_idx, area)
        }
    }

    match &app.overlay {
        Overlay::Help => {
            if let Some(help_area) = centered_rect(50, 45, area) {
                render_help_overlay(frame, t, help_area);
            }
        }
        Overlay::Error { message } => {
            if let Some(error_area) = centered_rect(50, 30, area) {
                render_error_overlay(frame, t, message, error_area);
            }
        }
        Overlay::RangeInput { max_input } => {
            if let Some(range_area) = centered_rect(35, 18, area) {
                render_range_input_overlay(frame, t, max_input, range_area);
            }
        }
        Overlay::Confirmation {
            action,
            position_name,
        } => {
            if let Some(confirm_area) = centered_rect(45, 22, area) {
                render_confirmation_overlay(frame, t, *action, position_name, confirm_area);
            }
        }
        Overlay::ThemeSelector { selected } => {
            if let Some(theme_area) = centered_fixed_rect(28, 15, area) {
                render_theme_selector(frame, t, *selected, theme_area);
            }
        }
        Overlay::None => {}
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
    let t = &app.theme;
    let status = match &app.connection {
        ConnectionState::Connected => Span::styled(" ● Connected ", Style::default().fg(t.success)),
        ConnectionState::Connecting => {
            Span::styled(" ◌ Connecting... ", Style::default().fg(t.warning))
        }
        ConnectionState::Disconnected { reason, .. } => Span::styled(
            format!(" ○ Disconnected: {} ", reason),
            Style::default().fg(t.error),
        ),
        ConnectionState::Reconnecting { attempt } => Span::styled(
            format!(" ◌ Reconnecting (attempt {})... ", attempt),
            Style::default().fg(t.warning),
        ),
    };

    let title = Line::from(vec![
        Span::styled(" Termion ", Style::default().bold().fg(t.text)),
        Span::raw("│"),
        status,
    ]);

    let header = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border)),
    );

    frame.render_widget(header, area);
}

struct RowData {
    idx: usize,
    position: String,
    device: String,
    flow_cell: String,
    state: String,
    run: String,
    reads: String,
    bases: String,
    throughput: String,
    error: String,
}

fn render_position_table(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let row_data: Vec<RowData> = app
        .positions
        .iter()
        .enumerate()
        .map(|(idx, pos)| {
            let run_state = app.run_states.get(&pos.name);
            let is_active = run_state.map(|s| s.is_active()).unwrap_or(false);

            let state_indicator = match run_state {
                Some(RunState::Running) => "● Running",
                Some(RunState::MuxScanning) => "◉ Pore Scan",
                Some(RunState::Paused) => "⏸ Paused",
                Some(RunState::Starting) => "◐ Starting",
                Some(RunState::Finishing) => "◑ Finishing",
                Some(RunState::Error(_)) => "✖ Error",
                Some(RunState::Idle) | None => match pos.state {
                    PositionState::Error => "✖ Error",
                    _ => "○ Idle",
                },
            };

            let stats = app.stats_cache.get(&pos.name);

            let reads = if is_active {
                stats
                    .map(|s| format_number(s.reads_processed))
                    .unwrap_or_else(|| "--".to_string())
            } else {
                "--".to_string()
            };

            let bases = if is_active {
                stats
                    .map(|s| format_bytes(s.bases_called))
                    .unwrap_or_else(|| "--".to_string())
            } else {
                "--".to_string()
            };

            let throughput = if is_active {
                stats
                    .map(|s| format_throughput_gbph(s.throughput_gbph))
                    .unwrap_or_else(|| "--".to_string())
            } else {
                "--".to_string()
            };

            let run_label = if is_active {
                app.run_info
                    .get(&pos.name)
                    .and_then(|info| info.display_label())
                    .unwrap_or_else(|| "--".to_string())
            } else {
                "--".to_string()
            };

            let device = pos.device_type.label().to_string();

            let flow_cell = app
                .flow_cell_info
                .get(&pos.name)
                .map(|fc| {
                    if fc.has_flow_cell {
                        fc.flow_cell_id.as_deref().unwrap_or("✓")
                    } else {
                        "✗"
                    }
                })
                .unwrap_or("--")
                .to_string();

            let error = match run_state {
                Some(RunState::Error(msg)) => msg.clone(),
                _ if pos.state == PositionState::Error => "Hardware error".to_string(),
                _ => String::new(),
            };

            RowData {
                idx,
                position: pos.name.clone(),
                device,
                flow_cell,
                state: state_indicator.to_string(),
                run: run_label,
                reads,
                bases,
                throughput,
                error,
            }
        })
        .collect();

    const PADDING: u16 = 2;
    let col_width = |header: &str, values: &[&str]| -> u16 {
        let max_content = values.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        max_content.max(header.len()) as u16 + PADDING
    };

    let headers = [
        "State",
        "Device",
        "Position",
        "FlowCell",
        "Run",
        "Reads",
        "Bases",
        "Throughput",
        "",
    ];

    let widths: Vec<Constraint> = headers
        .iter()
        .enumerate()
        .map(|(i, &h)| {
            let values: Vec<&str> = row_data
                .iter()
                .map(|r| match i {
                    0 => r.state.as_str(),
                    1 => r.device.as_str(),
                    2 => r.position.as_str(),
                    3 => r.flow_cell.as_str(),
                    4 => r.run.as_str(),
                    5 => r.reads.as_str(),
                    6 => r.bases.as_str(),
                    7 => r.throughput.as_str(),
                    8 => r.error.as_str(),
                    _ => "",
                })
                .collect();
            Constraint::Length(col_width(h, &values))
        })
        .collect();

    let header = Row::new(headers.to_vec())
        .style(Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = row_data
        .into_iter()
        .map(|r| {
            let style = if r.idx == app.selected_position {
                Style::default().bg(t.selection_bg).fg(t.selection_fg)
            } else {
                Style::default()
            };

            Row::new(vec![
                r.state,
                r.device,
                r.position,
                r.flow_cell,
                r.run,
                r.reads,
                r.bases,
                r.throughput,
                r.error,
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" Positions ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let hints = "[↑↓] Navigate  [Enter] Details  [q] Quit  [?] Help";

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(t.text_dim))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

fn render_detail_footer(frame: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let chart_hints = match app.detail_chart {
        DetailChart::Yield => "[t] Reads/Bases  ".to_string(),
        DetailChart::ReadLength => {
            let range_status = match app.histogram_range {
                Some((min, max)) => format!(" ({}-{} bp) [0] Clear", min, max),
                None => String::new(),
            };
            format!("[o] Outliers  [z] Set Range{}  ", range_status)
        }
        DetailChart::PoreActivity => String::new(),
    };

    let hints = format!("[Esc] Back  [1/2/3|Tab] Charts  {}[?] Help", chart_hints);

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(t.text_dim))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

fn render_position_detail(frame: &mut Frame, app: &App, position_idx: usize, area: Rect) {
    let t = &app.theme;
    let position = match app.positions.get(position_idx) {
        Some(p) => p,
        None => {
            let msg = Paragraph::new("Position not found")
                .style(Style::default().fg(t.error))
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

    let run_state = app.get_run_state(&position.name);
    render_detail_header(frame, t, position, run_state, chunks[0]);
    let histogram = app.histograms.get(&position.name);
    render_run_info(frame, t, position, stats, histogram, run_state, chunks[1]);

    match app.detail_chart {
        DetailChart::Yield => render_yield_chart(frame, app, &position.name, chunks[2]),
        DetailChart::ReadLength => {
            let histogram = app.histograms.get(&position.name);
            render_histogram_chart(
                frame,
                t,
                histogram,
                app.exclude_outliers,
                app.histogram_range,
                chunks[2],
            );
        }
        DetailChart::PoreActivity => {
            let channel_states = app.channel_states.get(&position.name);
            let channel_layout = app.channel_layouts.get(&position.name);
            render_pore_activity(frame, t, channel_states, channel_layout, chunks[2]);
        }
    }

    render_detail_footer(frame, app, chunks[3]);
}

fn render_detail_header(
    frame: &mut Frame,
    t: &Theme,
    position: &Position,
    run_state: Option<&RunState>,
    area: Rect,
) {
    let (state_color, state_indicator) = match run_state {
        Some(RunState::Running) => (t.success, "● Running"),
        Some(RunState::MuxScanning) => (t.special, "◉ Pore Scan"),
        Some(RunState::Paused) => (t.warning, "⏸ Paused"),
        Some(RunState::Starting) => (t.info, "◐ Starting"),
        Some(RunState::Finishing) => (t.warning, "◑ Finishing"),
        Some(RunState::Error(_)) => (t.error, "✖ Error"),
        Some(RunState::Idle) | None => match position.state {
            PositionState::Error => (t.error, "✖ Error"),
            _ => (t.idle, "○ Idle"),
        },
    };

    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", position.name),
            Style::default().bold().fg(t.text_title),
        ),
        Span::raw("── "),
        Span::styled(state_indicator, Style::default().fg(state_color).bold()),
    ]);

    let header = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border)),
    );

    frame.render_widget(header, area);
}

fn render_run_info(
    frame: &mut Frame,
    t: &Theme,
    _position: &Position,
    stats: Option<&StatsSnapshot>,
    histogram: Option<&ReadLengthHistogram>,
    run_state: Option<&RunState>,
    area: Rect,
) {
    let n50_text = histogram
        .filter(|h| h.n50 > 0.0)
        .map(|h| format!("{} bp", format_number(h.n50 as u64)))
        .unwrap_or_else(|| "-".to_string());

    let content = if let Some(s) = stats {
        vec![
            Line::from(vec![
                Span::styled("Reads: ", Style::default().fg(t.text_dim)),
                Span::styled(
                    format_number(s.reads_processed),
                    Style::default().bold().fg(t.text_title),
                ),
                Span::raw("  "),
                Span::styled(
                    format_number(s.reads_passed),
                    Style::default().fg(t.chart_passed),
                ),
                Span::styled(" passed  ", Style::default().fg(t.text_dim)),
                Span::styled(
                    format_number(s.reads_failed),
                    Style::default().fg(t.chart_failed),
                ),
                Span::styled(" failed", Style::default().fg(t.text_dim)),
            ]),
            Line::from(vec![
                Span::styled("Bases: ", Style::default().fg(t.text_dim)),
                Span::styled(
                    format_bytes(s.bases_called),
                    Style::default().bold().fg(t.text_title),
                ),
                Span::raw("  "),
                Span::styled(
                    format_bytes(s.bases_passed),
                    Style::default().fg(t.chart_passed),
                ),
                Span::styled(" passed  ", Style::default().fg(t.text_dim)),
                Span::styled(
                    format_bytes(s.bases_failed),
                    Style::default().fg(t.chart_failed),
                ),
                Span::styled(" failed", Style::default().fg(t.text_dim)),
            ]),
            Line::from(vec![
                Span::styled("Throughput: ", Style::default().fg(t.text_dim)),
                Span::styled(
                    match run_state {
                        Some(RunState::Running) => format_throughput_gbph(s.throughput_gbph),
                        _ => "--".to_string(),
                    },
                    Style::default().bold(),
                ),
                Span::raw("    "),
                Span::styled("Pass Rate: ", Style::default().fg(t.text_dim)),
                Span::styled(format!("{:.1}%", s.pass_rate()), Style::default().bold()),
                Span::raw("    "),
                Span::styled("Active Pores: ", Style::default().fg(t.text_dim)),
                Span::styled(
                    match run_state {
                        Some(RunState::Running) => format_number(s.active_pores as u64),
                        _ => "--".to_string(),
                    },
                    Style::default().bold(),
                ),
                Span::raw("    "),
                Span::styled("N50: ", Style::default().fg(t.text_dim)),
                Span::styled(n50_text, Style::default().bold()),
            ]),
        ]
    } else {
        vec![Line::from("No data available")]
    };

    let info = Paragraph::new(content).block(
        Block::default()
            .title(" Run Info ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border)),
    );

    frame.render_widget(info, area);
}

fn render_yield_chart(frame: &mut Frame, app: &App, position_name: &str, area: Rect) {
    let t = &app.theme;
    let yield_data = app.yield_history.get(position_name);

    let yield_points = match yield_data {
        Some(points) if !points.is_empty() => points,
        _ => {
            let placeholder = Paragraph::new("Waiting for data...")
                .style(Style::default().fg(t.text_dim))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Cumulative Yield ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(t.border)),
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
            .style(Style::default().fg(t.chart_failed))
            .data(&failed_data),
        Dataset::default()
            .name("Passed")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(t.chart_passed))
            .data(&passed_data),
        Dataset::default()
            .name("Total")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(t.chart_line))
            .data(&total_data),
    ];

    let time_label = format_time_label(max_x - min_x);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(t.chart_axis))
                .bounds([0.0, max_x - min_x])
                .labels(vec![Line::from("0"), Line::from(time_label)]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(t.chart_axis))
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
    t: &Theme,
    histogram: Option<&ReadLengthHistogram>,
    exclude_outliers: bool,
    histogram_range: Option<(u64, u64)>,
    area: Rect,
) {
    let base_title = " Read Length Distribution ";
    let title = match (exclude_outliers, histogram_range) {
        (true, Some((min, max))) => format!(
            " Read Length ({}-{} bp, outliers excluded) ",
            format_number(min),
            format_number(max)
        ),
        (false, Some((min, max))) => format!(
            " Read Length ({}-{} bp) ",
            format_number(min),
            format_number(max)
        ),
        (true, None) => " Read Length Distribution (outliers excluded) ".to_string(),
        (false, None) => base_title.to_string(),
    };

    let histogram = match histogram {
        Some(h) if !h.bucket_values.is_empty() => h,
        _ => {
            let placeholder = Paragraph::new("Waiting for histogram data...")
                .style(Style::default().fg(t.text_dim))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(t.special)),
                );
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let max_count = histogram.max_value();
    let num_buckets = histogram.bucket_values.len();

    let y_max_rounded = if max_count > 1000 {
        ((max_count / 1000) + 1) * 1000
    } else if max_count > 100 {
        ((max_count / 100) + 1) * 100
    } else {
        ((max_count / 10) + 1) * 10
    }
    .max(10);

    let x_min = histogram
        .bucket_ranges
        .first()
        .map(|(s, _)| *s)
        .unwrap_or(0);
    let x_max = histogram
        .bucket_ranges
        .last()
        .map(|(_, e)| *e)
        .unwrap_or(10000);

    let range_label = format!(" {} - {} bp ", format_number(x_min), format_number(x_max));

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(7), Constraint::Min(10)])
        .split(area);

    let y_axis_area = layout[0];
    let chart_area = layout[1];

    let chart_height = chart_area.height.saturating_sub(3) as usize;
    let mut y_axis_lines: Vec<Line> = Vec::new();

    y_axis_lines.push(Line::from(""));

    if chart_height >= 6 {
        y_axis_lines.push(Line::from(Span::styled(
            format!("{:>6}", format_number(y_max_rounded)),
            Style::default().fg(t.chart_axis),
        )));
        for _ in 0..(chart_height.saturating_sub(3) / 2) {
            y_axis_lines.push(Line::from(""));
        }
        y_axis_lines.push(Line::from(Span::styled(
            format!("{:>6}", format_number(y_max_rounded / 2)),
            Style::default().fg(t.chart_axis),
        )));
        for _ in 0..(chart_height.saturating_sub(3) / 2) {
            y_axis_lines.push(Line::from(""));
        }
        y_axis_lines.push(Line::from(Span::styled(
            format!("{:>6}", "0"),
            Style::default().fg(t.chart_axis),
        )));
    } else {
        y_axis_lines.push(Line::from(Span::styled(
            format!("{:>6}", format_number(y_max_rounded)),
            Style::default().fg(t.chart_axis),
        )));
        y_axis_lines.push(Line::from(""));
        y_axis_lines.push(Line::from(Span::styled(
            format!("{:>6}", "0"),
            Style::default().fg(t.chart_axis),
        )));
    }

    let y_axis = Paragraph::new(y_axis_lines).alignment(Alignment::Right);
    frame.render_widget(y_axis, y_axis_area);

    let inner_width = chart_area.width.saturating_sub(2) as usize;
    let total_space_per_bar = inner_width / num_buckets;
    let bar_width: u16 = total_space_per_bar.saturating_sub(1).clamp(1, 4) as u16;
    let bar_gap: u16 = 1;

    let bars: Vec<Bar> = histogram
        .bucket_values
        .iter()
        .map(|&count| {
            Bar::default()
                .value(count)
                .label(Line::from(""))
                .text_value(String::new())
                .style(Style::default().fg(t.special))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(title)
                .title_bottom(Line::from(range_label).alignment(Alignment::Center))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.special)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .max(y_max_rounded);

    frame.render_widget(bar_chart, chart_area);
}

fn render_pore_activity(
    frame: &mut Frame,
    t: &Theme,
    channel_states: Option<&ChannelStatesSnapshot>,
    channel_layout: Option<&ChannelLayout>,
    area: Rect,
) {
    let title = " Pore Activity [3] ";

    let channel_states = match channel_states {
        Some(cs) if !cs.states.is_empty() => cs,
        _ => {
            let placeholder = Paragraph::new("Waiting for channel data...")
                .style(Style::default().fg(t.text_dim))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(t.warning)),
                );
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_pore_grid_from_states(frame, t, channel_states, channel_layout, chunks[0]);
    render_state_counts(frame, t, channel_states, chunks[1]);
}

fn render_pore_grid_from_states(
    frame: &mut Frame,
    t: &Theme,
    channel_states: &ChannelStatesSnapshot,
    channel_layout: Option<&ChannelLayout>,
    area: Rect,
) {
    let inner = Block::default()
        .title(" Channel Map ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.warning));

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
                    state_to_symbol(t, &channel_states.states[idx])
                }
                _ => ("  ", t.channel_empty),
            };
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
        grid_row = (grid_row + 1).min(grid_height.saturating_sub(1));
    }

    let grid = Paragraph::new(lines);
    frame.render_widget(grid, inner_area);
}

fn render_state_counts(
    frame: &mut Frame,
    t: &Theme,
    channel_states: &ChannelStatesSnapshot,
    area: Rect,
) {
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
        ("Sequencing", sequencing, t.channel_sequencing),
        ("Pore Available", pore_available, t.channel_pore),
        ("Unavailable", unavailable, t.channel_unavailable),
        ("Inactive", inactive, t.channel_inactive),
        ("Other", other, t.channel_other),
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
            Span::styled(format!("{:5.1}%", percent), Style::default().fg(t.text_dim)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Total: ", Style::default().fg(t.text_dim)),
        Span::styled(format!("{}", total), Style::default().bold()),
        Span::raw(" channels"),
    ]));

    let breakdown = Paragraph::new(lines).block(
        Block::default()
            .title(" Statistics ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.warning)),
    );

    frame.render_widget(breakdown, area);
}

fn state_to_symbol(t: &Theme, state: &str) -> (&'static str, Color) {
    let s = state.to_lowercase();
    if s.contains("strand") || s.contains("sequencing") {
        ("██", t.channel_sequencing)
    } else if s.contains("pore") || s.contains("single") {
        ("██", t.channel_pore)
    } else if s.contains("unavailable") || s.contains("saturated") {
        ("░░", t.channel_unavailable)
    } else if s.contains("inactive") || s.contains("zero") || s.contains("multiple") {
        ("░░", t.channel_inactive)
    } else if s.contains("adapter") || s.contains("event") {
        ("▓▓", t.channel_adapter)
    } else if s.contains("unblock") {
        ("▒▒", t.channel_adapter)
    } else if s.is_empty() || s == "unknown" {
        ("  ", t.channel_empty)
    } else {
        ("░░", t.channel_other)
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

fn render_help_overlay(frame: &mut Frame, t: &Theme, area: Rect) {
    let key_style = Style::default().fg(t.key_hint).bold();
    let desc_style = Style::default().fg(t.text);
    let section_style = Style::default().fg(t.text_title).bold();
    let dim_style = Style::default().fg(t.text_dim);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("─── ", dim_style),
            Span::styled("Navigation", section_style),
            Span::styled(" ───", dim_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑ ↓ ", key_style),
            Span::styled("Move", desc_style),
            Span::styled("   Enter ", key_style),
            Span::styled("Select", desc_style),
            Span::styled("   Esc ", key_style),
            Span::styled("Back", desc_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("─── ", dim_style),
            Span::styled("Charts", section_style),
            Span::styled(" ───", dim_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1 ", key_style),
            Span::styled("Yield", desc_style),
            Span::styled("   2 ", key_style),
            Span::styled("Read Length", desc_style),
            Span::styled("   3 ", key_style),
            Span::styled("Pore Activity", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab ", key_style),
            Span::styled("Cycle charts", desc_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("─── ", dim_style),
            Span::styled("Run Control", section_style),
            Span::styled(" ───", dim_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  p ", key_style),
            Span::styled("Pause", desc_style),
            Span::styled("   r ", key_style),
            Span::styled("Resume", desc_style),
            Span::styled("   s ", key_style),
            Span::styled("Stop", desc_style),
            Span::styled("   R ", key_style),
            Span::styled("Refresh", desc_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("───────────────────────", dim_style)]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  T ", key_style),
            Span::styled("Theme", desc_style),
            Span::styled("   ? ", key_style),
            Span::styled("Help", desc_style),
            Span::styled("   q ", key_style),
            Span::styled("Quit", desc_style),
        ]),
        Line::from(""),
    ];

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Keyboard Shortcuts ")
                .title_style(Style::default().fg(t.text_title).bold())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(t.border_dim))
                .style(Style::default().bg(t.background)),
        );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(help, area);
}

fn render_error_overlay(frame: &mut Frame, t: &Theme, message: &str, area: Rect) {
    let error = Paragraph::new(vec![
        Line::from(Span::styled("Error", Style::default().bold().fg(t.error))),
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(Span::styled("[Esc] Close", Style::default().fg(t.text_dim))),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.error))
            .style(Style::default().bg(t.background)),
    );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(error, area);
}

fn render_range_input_overlay(frame: &mut Frame, t: &Theme, max_input: &str, area: Rect) {
    let max_display = if max_input.is_empty() {
        "(empty = full range)".to_string()
    } else {
        format!("{} bp", max_input)
    };

    let content = vec![
        Line::from(Span::styled(
            "Set Max Read Length",
            Style::default().bold().fg(t.text_title),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Max: ", Style::default().fg(t.text_dim)),
            Span::styled(max_display, Style::default().fg(t.key_hint).bold()),
            Span::styled("_", Style::default().fg(t.key_hint)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Type number | ↑/↓ ±1000 | Enter | Esc",
            Style::default().fg(t.text_dim),
        )),
    ];

    let dialog = Paragraph::new(content).alignment(Alignment::Center).block(
        Block::default()
            .title(" Range ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.special))
            .style(Style::default().bg(t.background)),
    );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(dialog, area);
}

fn render_confirmation_overlay(
    frame: &mut Frame,
    t: &Theme,
    action: RunControlAction,
    position_name: &str,
    area: Rect,
) {
    let (title_color, border_color) = match action {
        RunControlAction::Stop => (t.error, t.error),
        RunControlAction::Pause => (t.warning, t.warning),
        RunControlAction::Resume => (t.success, t.success),
    };

    let content = vec![
        Line::from(Span::styled(
            format!("{} Run", action.label()),
            Style::default().bold().fg(title_color),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Position: {}", position_name),
            Style::default().fg(t.text_title),
        )),
        Line::from(""),
        Line::from(action.confirmation_message()),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(t.key_hint).bold()),
            Span::styled("Confirm", Style::default()),
            Span::raw("    "),
            Span::styled("[Esc] ", Style::default().fg(t.text_dim).bold()),
            Span::styled("Cancel", Style::default().fg(t.text_dim)),
        ]),
    ];

    let dialog = Paragraph::new(content).alignment(Alignment::Center).block(
        Block::default()
            .title(format!(" {} ", action.label()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(t.background)),
    );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(dialog, area);
}

fn render_theme_selector(frame: &mut Frame, t: &Theme, selected: usize, area: Rect) {
    use super::theme::Theme as ThemeType;

    let themes = ThemeType::available_themes();
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Select Theme",
            Style::default().bold().fg(t.text_title),
        )),
        Line::from(""),
    ];

    for (idx, &name) in themes.iter().enumerate() {
        let display_name = match name {
            "default" => "Default",
            "catppuccin" => "Catppuccin Mocha",
            "dracula" => "Dracula",
            "tokyo-night" => "Tokyo Night",
            "gruvbox" => "Gruvbox",
            "nord" => "Nord",
            "neon" => "Neon",
            other => other,
        };

        let is_selected = idx == selected;
        let prefix = if is_selected { "► " } else { "  " };
        let style = if is_selected {
            Style::default().fg(t.text).bold()
        } else {
            Style::default().fg(t.text_dim)
        };

        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, display_name),
            style,
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[↑↓] ", Style::default().fg(t.key_hint).bold()),
        Span::styled("Select", Style::default().fg(t.text_dim)),
        Span::raw("  "),
        Span::styled("[Enter] ", Style::default().fg(t.key_hint).bold()),
        Span::styled("Apply", Style::default().fg(t.text_dim)),
        Span::raw("  "),
        Span::styled("[Esc] ", Style::default().fg(t.text_dim).bold()),
        Span::styled("Cancel", Style::default().fg(t.text_dim)),
    ]));

    let dialog = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .title(" Theme ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border))
            .style(Style::default().bg(t.background)),
    );

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(dialog, area);
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

fn centered_fixed_rect(width: u16, height: u16, area: Rect) -> Option<Rect> {
    if area.width < width || area.height < height {
        return None;
    }

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Some(Rect::new(x, y, width, height))
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        let val = n as f64 / 1_000_000_000.0;
        if val.fract() == 0.0 {
            format!("{}B", val as u64)
        } else {
            format!("{:.1}B", val)
        }
    } else if n >= 1_000_000 {
        let val = n as f64 / 1_000_000.0;
        if val.fract() == 0.0 {
            format!("{}M", val as u64)
        } else {
            format!("{:.1}M", val)
        }
    } else if n >= 1_000 {
        let val = n as f64 / 1_000.0;
        if val.fract() == 0.0 {
            format!("{}K", val as u64)
        } else {
            format!("{:.1}K", val)
        }
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

fn format_throughput_gbph(gbph: f64) -> String {
    if gbph <= 0.0 {
        "--".to_string()
    } else if gbph >= 1.0 {
        format!("{:.2} Gb/h", gbph)
    } else if gbph >= 0.001 {
        format!("{:.2} Mb/h", gbph * 1_000.0)
    } else {
        format!("{:.2} Kb/h", gbph * 1_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_cell_type_from_channel_count_minion_512() {
        assert_eq!(FlowCellType::from_channel_count(512), FlowCellType::MinION);
    }

    #[test]
    fn test_flow_cell_type_from_channel_count_promethion_3000() {
        assert_eq!(
            FlowCellType::from_channel_count(3000),
            FlowCellType::PromethION
        );
    }

    #[test]
    fn test_flow_cell_type_from_channel_count_flongle_126() {
        assert_eq!(FlowCellType::from_channel_count(126), FlowCellType::MinION);
    }

    #[test]
    fn test_flow_cell_type_from_channel_count_zero() {
        assert_eq!(FlowCellType::from_channel_count(0), FlowCellType::MinION);
    }

    #[test]
    fn test_flow_cell_type_from_channel_count_boundary_513() {
        assert_eq!(
            FlowCellType::from_channel_count(513),
            FlowCellType::PromethION
        );
    }

    #[test]
    fn test_flow_cell_type_default() {
        assert_eq!(FlowCellType::default(), FlowCellType::MinION);
    }

    #[test]
    fn test_block_arrangement_variants() {
        let two_vert = BlockArrangement::TwoVertical;
        let four_quad = BlockArrangement::FourQuadrant;
        assert_ne!(two_vert, four_quad);
    }

    #[test]
    fn test_gap_position_horizontal() {
        let gap = GapPosition::Horizontal { after_row: 7 };
        assert_eq!(gap, GapPosition::Horizontal { after_row: 7 });
    }

    #[test]
    fn test_gap_position_vertical() {
        let gap = GapPosition::Vertical { after_col: 15 };
        assert_eq!(gap, GapPosition::Vertical { after_col: 15 });
    }

    #[test]
    fn test_grid_structure_creation() {
        let grid = GridStructure {
            flow_cell_type: FlowCellType::MinION,
            grid_cols: 32,
            grid_rows: 16,
            block_arrangement: BlockArrangement::TwoVertical,
            gap_positions: vec![GapPosition::Horizontal { after_row: 7 }],
            cell_width: 2,
        };

        assert_eq!(grid.flow_cell_type, FlowCellType::MinION);
        assert_eq!(grid.grid_cols, 32);
        assert_eq!(grid.grid_rows, 16);
        assert_eq!(grid.block_arrangement, BlockArrangement::TwoVertical);
        assert_eq!(grid.gap_positions.len(), 1);
        assert_eq!(grid.cell_width, 2);
    }
}
