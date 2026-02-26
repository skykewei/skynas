mod cli;
mod config;
mod converter;
mod db;
mod mdns;
mod models;
mod notify;
mod qr;
mod server;
mod sync;
mod websocket;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use db::Database;
use qr::{get_best_host, print_server_info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Handle subcommands
    match cli.command {
        Some(Commands::Start { background }) => {
            if background {
                println!("Starting SkyNAS in background mode...");
                // TODO: Implement daemonization
            }
            run_server(cli.port).await
        }
        Some(Commands::Stop) => {
            println!("Stopping SkyNAS...");
            // TODO: Implement stop signal
            Ok(())
        }
        Some(Commands::Status) => {
            println!("Checking SkyNAS status...");
            // TODO: Check if server is running
            Ok(())
        }
        Some(Commands::MenuBar) => {
            println!("Starting SkyNAS menu bar app...");
            // TODO: Implement menu bar
            Ok(())
        }
        None => {
            // Default: run server interactively
            run_server(cli.port).await
        }
    }
}

async fn run_server(port_override: Option<u16>) -> anyhow::Result<()> {
    let mut config = Config::load()?;

    // Override port if specified via CLI
    if let Some(port) = port_override {
        config.server.port = port;
    }

    let db = Database::new(&config.storage.db_path)?;

    let host = get_best_host(&config.server.host);
    print_server_info(&host, config.server.port);

    // Start mDNS in a separate thread if enabled
    if config.features.mdns_enabled {
        let port = config.server.port;
        std::thread::spawn(move || {
            let device_id = mdns::generate_device_id();
            let service_name = format!("SkyNAS-{}", device_id);

            let mut mdns = mdns::MdnsPublisher::new();
            if let Err(e) = mdns.publish(&service_name, port, &device_id) {
                eprintln!("Failed to start mDNS: {}", e);
            }
        });
    }

    // Run server
    server::run_server(config, db).await?;

    Ok(())
}
