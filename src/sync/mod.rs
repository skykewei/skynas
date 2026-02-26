use crate::config::Config;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, sleep};

pub struct SyncManager {
    config: Config,
}

impl SyncManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn sync_to_cloud(&self) -> anyhow::Result<bool> {
        if !self.config.sync.enabled {
            return Ok(false);
        }

        if let Some(command) = &self.config.sync.command {
            println!("Executing sync command: {}", command);

            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.config.storage.base_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            if output.status.success() {
                println!("Sync completed successfully");
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Sync failed: {}", stderr);
                Ok(false)
            }
        } else {
            println!("No sync command configured");
            Ok(false)
        }
    }

    pub async fn sync_with_delay(&self, delay_secs: u64) -> anyhow::Result<bool> {
        sleep(Duration::from_secs(delay_secs)).await;
        self.sync_to_cloud().await
    }

    pub fn spawn_sync_task(&self, delay_secs: u64) -> tokio::task::JoinHandle<bool> {
        let config = self.config.clone();
        tokio::spawn(async move {
            let manager = SyncManager::new(config);
            manager.sync_with_delay(delay_secs).await.unwrap_or(false)
        })
    }
}
