mod config;
mod db;
mod models;
mod qr;
mod server;

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

    // Run server
    server::run_server(config, db).await?;

    Ok(())
}
