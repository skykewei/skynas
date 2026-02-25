mod config;
mod db;
mod models;

use config::Config;
use db::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
    let _db = Database::new(&config.storage.db_path)?;

    println!("Database initialized successfully");
    println!("Storage path: {:?}", config.storage.base_path);

    Ok(())
}
