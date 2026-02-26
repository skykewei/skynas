<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/icon.svg">
  <img src="assets/icon.svg" alt="SkyNAS Logo" width="180" height="180">
</picture>

# SkyNAS Photo Sync

<h3>Sync photos from iPhone to Mac with ease</h3>

<p>
  <a href="https://github.com/skykewei/skynas/releases/latest">
    <img src="https://img.shields.io/github/v/release/skykewei/skynas?style=for-the-badge&logo=github&color=blue" alt="Release">
  </a>
  <a href="https://github.com/skykewei/skynas/actions">
    <img src="https://github.com/skykewei/skynas/actions/workflows/ci.yml/badge.svg?style=for-the-badge" alt="CI">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/github/license/skykewei/skynas?style=for-the-badge&color=green" alt="License">
  </a>
  <a href="https://github.com/skykewei/skynas/stargazers">
    <img src="https://img.shields.io/github/stars/skykewei/skynas?style=for-the-badge&color=yellow" alt="Stars">
  </a>
</p>

<p>
  <img src="https://img.shields.io/github/downloads/skykewei/skynas/total?style=for-the-badge&color=purple" alt="Downloads">
  <img src="https://img.shields.io/github/languages/code-size/skykewei/skynas?style=for-the-badge&color=orange" alt="Code Size">
  <img src="https://img.shields.io/github/last-commit/skykewei/skynas?style=for-the-badge&color=red" alt="Last Commit">
</p>

<p>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey?style=for-the-badge&logo=apple" alt="Platform">
</p>

<p>
  <strong>🇺🇸 English</strong> •
  <a href="docs/README.zh.md">🇨🇳 中文</a>
</p>

</div>

---

## ✨ Features

- **📱 No App Required** - iPhone users access via web browser
- **📷 QR Code Connection** - Scan to connect instantly
- **📁 Album Organization** - Photos sorted by albums automatically
- **🔄 Chunked Upload** - Resume interrupted transfers
- **🔍 Duplicate Detection** - SHA256-based deduplication
- **🖼️ HEIC to JPEG** - Multiple conversion backends (sips, image, libheif)
- **☁️ Auto Cloud Sync** - Sync to NAS/Cloud after upload
- **📊 Real-time Progress** - WebSocket live updates
- **🔎 mDNS Discovery** - Auto-discover on local network
- **🔔 Native Notifications** - macOS system notifications

---

## 📦 Installation

### Homebrew (Recommended)

```bash
brew tap skykewei/skynas
brew install skynas
```

### Binary Download

Download from [Releases](https://github.com/skykewei/skynas/releases):

```bash
# Intel Mac
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-x86_64.tar.gz

# Apple Silicon
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-arm64.tar.gz

# Universal Binary (Recommended)
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-universal.tar.gz

# Install
tar -xzf skynas.tar.gz
sudo mv skynas-*/skynas /usr/local/bin/
```

### macOS App

Download `SkyNAS-x.x.x.zip` from [Releases](https://github.com/skykewei/skynas/releases), extract and drag `SkyNAS.app` to `/Applications`.

---

## 🚀 Quick Start

```bash
# Start server (default port 8080)
skynas

# Custom port
skynas --port 8081

# Background daemon
skynas start --background

# Check status
skynas status

# Stop daemon
skynas stop
```

---

## ⚙️ Configuration

Create `~/.config/skynas/config.toml`:

```toml
[server]
port = 8080
host = "0.0.0.0"

[storage]
base_path = "/Users/$USER/Pictures/iPhoneSync"

[sync]
enabled = true
auto_sync = true
command = "rclone sync ~/Pictures/iPhoneSync nas:Photos"

[heic_converter]
backend = "sips"  # Options: sips, image, libheif
generate_jpeg = true
jpeg_quality = 85
```

---

## 🌐 Using with iPhone

1. Start SkyNAS on your Mac
2. Scan the QR code displayed in terminal
3. Open the link on iPhone Safari
4. Select photos and upload
5. Photos appear organized in Mac's Pictures folder

---

## 💻 CLI Commands

```bash
skynas [OPTIONS] [COMMAND]

Commands:
  start      Start the server
  stop       Stop the running server
  status     Show server status
  menu-bar   Run as menu bar app (macOS only)
  help       Print help

Options:
  -c, --config <CONFIG>  Configuration file path
  -p, --port <PORT>      Server port
  -d, --daemon           Run in daemon mode
  -h, --help             Print help
```

---

## 🌐 API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Web upload interface |
| `/ws` | GET | WebSocket for real-time updates |
| `/api/upload` | POST | Simple file upload |
| `/api/upload/chunked/init` | POST | Initialize chunked upload |
| `/api/upload/chunked/chunk` | POST | Upload chunk |
| `/api/upload/chunked/complete/:id` | POST | Complete chunked upload |
| `/api/upload/chunked/status/:id` | GET | Check upload status |
| `/api/health` | GET | Health check |

---

## 🔧 Development

```bash
# Clone repository
git clone https://github.com/skykewei/skynas.git
cd skynas

# Build
cargo build --release

# Run tests
cargo test

# Check clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

---

## 📊 Tech Stack

<p align="center">
  <img src="https://img.shields.io/badge/Rust-DEA584?style=for-the-badge&logo=rust&logoColor=black" alt="Rust">
  <img src="https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Tokio">
  <img src="https://img.shields.io/badge/Axum-7B68EE?style=for-the-badge&logo=rust&logoColor=white" alt="Axum">
  <img src="https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite">
  <img src="https://img.shields.io/badge/WebSocket-010101?style=for-the-badge&logo=socket.io&logoColor=white" alt="WebSocket">
</p>

- **HTTP Server**: Axum (async web framework)
- **Database**: SQLite with rusqlite
- **Runtime**: Tokio async runtime
- **mDNS**: Zeroconf for service discovery
- **Notifications**: notify-rust for macOS native notifications
- **Image Processing**: image crate, sips, libheif

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">
  <p>
    Made with ❤️ by <a href="https://github.com/skykewei">skykewei</a>
  </p>
  <p>
    <a href="https://github.com/skykewei/skynas/stargazers">
      <img src="https://img.shields.io/github/stars/skykewei/skynas?style=social" alt="Stars">
    </a>
  </p>
</div>
