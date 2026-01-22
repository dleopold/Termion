//! `termion status` command implementation.

use crate::client::Client;
use crate::config::Config;

pub async fn run(
    config: &Config,
    json: bool,
    position_filter: Option<String>,
) -> anyhow::Result<()> {
    let mut client = Client::connect_with_timeouts(
        &config.connection.host,
        config.connection.port,
        config.connection.connect_timeout,
        config.connection.request_timeout,
    )
    .await?;
    let positions = client.list_positions().await?;

    if positions.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No positions found");
        }
        return Ok(());
    }

    let positions_to_check: Vec<_> = if let Some(ref filter) = position_filter {
        positions
            .into_iter()
            .filter(|p| p.name == *filter)
            .collect()
    } else {
        positions
    };

    if positions_to_check.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "Position '{}' not found",
                position_filter.unwrap_or_default()
            );
        }
        return Ok(());
    }

    let mut results = Vec::new();

    for position in positions_to_check {
        let status = if position.grpc_port > 0 {
            match client.connect_position(position.clone()).await {
                Ok(mut pos_client) => match pos_client.get_acquisition_info().await {
                    Ok(info) => {
                        let has_run = info.state.has_displayable_run();
                        PositionStatus {
                            name: position.name.clone(),
                            state: info.state.label().to_string(),
                            run_id: if info.run_id.is_empty() || !has_run {
                                None
                            } else {
                                Some(info.run_id)
                            },
                            reads: if has_run { info.reads_processed } else { 0 },
                            bases_passed: if has_run { info.bases_passed } else { 0 },
                            bases_failed: if has_run { info.bases_failed } else { 0 },
                            simulated: position.is_simulated,
                        }
                    }
                    Err(e) => PositionStatus {
                        name: position.name.clone(),
                        state: format!("Error: {}", e),
                        run_id: None,
                        reads: 0,
                        bases_passed: 0,
                        bases_failed: 0,
                        simulated: position.is_simulated,
                    },
                },
                Err(e) => PositionStatus {
                    name: position.name.clone(),
                    state: format!("Connection error: {}", e),
                    run_id: None,
                    reads: 0,
                    bases_passed: 0,
                    bases_failed: 0,
                    simulated: position.is_simulated,
                },
            }
        } else {
            PositionStatus {
                name: position.name.clone(),
                state: "Not running".to_string(),
                run_id: None,
                reads: 0,
                bases_passed: 0,
                bases_failed: 0,
                simulated: position.is_simulated,
            }
        };

        results.push(status);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for status in results {
            println!(
                "Position: {}{}",
                status.name,
                if status.simulated { " (simulated)" } else { "" }
            );
            println!("  State: {}", status.state);
            if let Some(run_id) = &status.run_id {
                println!("  Run ID: {}", run_id);
            }
            println!("  Reads: {}", format_number(status.reads));
            println!("  Bases passed: {}", format_bases(status.bases_passed));
            println!("  Bases failed: {}", format_bases(status.bases_failed));
            println!();
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct PositionStatus {
    name: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    reads: u64,
    bases_passed: u64,
    bases_failed: u64,
    simulated: bool,
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bases(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2} Gb", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2} Mb", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2} Kb", n as f64 / 1_000.0)
    } else {
        format!("{} b", n)
    }
}
