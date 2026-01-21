//! `termion list` command implementation.

use crate::client::Client;
use crate::config::Config;

pub async fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let mut client = Client::connect_with_timeouts(
        &config.connection.host,
        config.connection.port,
        config.connection.connect_timeout,
        config.connection.request_timeout,
    )
    .await?;
    let devices = client.list_devices().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&devices)?);
    } else if devices.is_empty() {
        println!("No devices found");
    } else {
        for device in devices {
            println!("{}: {} ({:?})", device.id, device.name, device.state);
        }
    }

    Ok(())
}
