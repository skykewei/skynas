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

use config::Config;
use db::Database;
use qr::{print_server_info, get_best_host};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
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
