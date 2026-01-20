//! `termion list` command implementation.

use crate::client::Client;
use crate::config::Config;

pub async fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let mut client = Client::connect(&config.connection.host, config.connection.port).await?;
    let devices = client.list_devices().await?;

    if json {
        println!("[]");
    } else if devices.is_empty() {
        println!("No devices found");
    } else {
        for device in devices {
            println!("{}: {} ({:?})", device.id, device.name, device.state);
        }
    }

    Ok(())
}
