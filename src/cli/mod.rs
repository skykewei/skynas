use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skynas")]
#[command(about = "SkyNAS Photo Sync - Sync photos from iPhone to Mac")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<String>,

    /// Server port
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Run in daemon mode
    #[arg(short, long)]
    pub daemon: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the server
    Start {
        /// Run in background
        #[arg(short, long)]
        background: bool,
    },

    /// Stop the running server
    Stop,

    /// Show server status
    Status,

    /// Run as menu bar app (macOS only)
    MenuBar,
}
