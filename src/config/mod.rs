use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub sync: SyncConfig,
    pub heic_converter: HeicConverterConfig,
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub max_upload_size: usize, // in bytes
    pub chunk_size: usize,      // in bytes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub base_path: PathBuf,
    pub db_path: PathBuf,
    pub default_album: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub enabled: bool,
    pub command: Option<String>,
    pub auto_sync: bool,
    pub sync_delay_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeicConverterConfig {
    pub backend: String, // "image", "libheif", "sips"
    pub generate_jpeg: bool,
    pub jpeg_quality: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub mdns_enabled: bool,
    pub websocket_enabled: bool,
    pub notification_enabled: bool,
    pub qr_code_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let base_path = home_dir.join("Pictures").join("iPhoneSync");

        Self {
            server: ServerConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
                max_upload_size: 100 * 1024 * 1024, // 100MB
                chunk_size: 1024 * 1024,            // 1MB chunks
            },
            storage: StorageConfig {
                base_path: base_path.clone(),
                db_path: base_path.join(".skynas").join("skynas.db"),
                default_album: "未分类".to_string(),
            },
            sync: SyncConfig {
                enabled: false,
                command: None,
                auto_sync: false,
                sync_delay_seconds: 5,
            },
            heic_converter: HeicConverterConfig {
                backend: "image".to_string(),
                generate_jpeg: true,
                jpeg_quality: 85,
            },
            features: FeaturesConfig {
                mdns_enabled: true,
                websocket_enabled: true,
                notification_enabled: true,
                qr_code_enabled: true,
            },
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        // Try to load user config
        if let Some(config_dir) = dirs::config_dir() {
            let user_config_path = config_dir.join("skynas").join("config.toml");
            if user_config_path.exists() {
                let content = std::fs::read_to_string(&user_config_path)?;
                let user_config: toml::Value = toml::from_str(&content)?;

                // Merge user config with default (simplified - can use config crate for deep merge)
                // For now, just return default
            }
        }

        // Override with environment variables if needed
        if let Ok(port) = std::env::var("SKYNAS_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                config.server.port = port_num;
            }
        }

        // Ensure directories exist
        std::fs::create_dir_all(&config.storage.base_path)?;
        std::fs::create_dir_all(config.storage.db_path.parent().unwrap())?;

        Ok(config)
    }

    pub fn server_url(&self) -> String {
        format!("http://{}:{}", self.server.host, self.server.port)
    }
}
