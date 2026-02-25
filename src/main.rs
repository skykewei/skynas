mod config;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::load()?;
    println!("Config loaded: {:?}", config);

    Ok(())
}
