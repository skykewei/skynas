use crate::config::Config;

pub fn print_startup_info(config: &Config) {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║                    SkyNAS Server                         ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  Server:     http://{}:{}", config.server.host, config.server.port);
    println!("║                                                          ║");
    println!("║  Storage:    {}", config.storage.base_path.display());
    println!("║  Database:   {}", config.storage.db_path.display());
    println!("║                                                          ║");
    println!("║  Features:                                               ║");
    println!("║    - mDNS:        {}", feature_status(config.features.mdns_enabled));
    println!("║    - WebSocket:   {}", feature_status(config.features.websocket_enabled));
    println!("║    - HEIC→JPEG:   {}", feature_status(config.heic_converter.generate_jpeg));
    println!("║    - Sync Cmd:    {}", feature_status(config.sync.enabled));
    println!("╚══════════════════════════════════════════════════════════╝");
}

fn feature_status(enabled: bool) -> &'static str {
    if enabled { "✓ enabled" } else { "✗ disabled" }
}
