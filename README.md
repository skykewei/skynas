# SkyNAS Photo Sync

A Rust-based photo sync tool that allows iPhone users to quickly sync photos to Mac via web browser.

## Features

- **Simple Web Interface**: No app installation needed on iPhone
- **QR Code Connection**: Scan to connect, no typing URLs
- **Album Organization**: Photos are organized by album
- **Chunked Upload with Resume**: Reliable large file transfer
- **Duplicate Detection**: SHA256 hash-based deduplication
- **HEIC to JPEG Conversion**: Configurable backends (sips, image, libheif)
- **Auto Cloud Sync**: Sync to NAS/Cloud after upload
- **Real-time Progress**: WebSocket notifications
- **mDNS Discovery**: Auto-discover Mac on local network

## Installation

### Homebrew (推荐)

```bash
brew tap skykewei/skynas
brew install skynas
```

### GitHub Release

从 [Releases](https://github.com/skykewei/skynas/releases) 下载对应架构的二进制文件：

```bash
# Intel Mac
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-x86_64.tar.gz

# Apple Silicon M1/M2/M3
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-arm64.tar.gz

# Universal Binary (两种架构都支持)
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-universal.tar.gz

# 解压安装
tar -xzf skynas.tar.gz
sudo mv skynas-*/skynas /usr/local/bin/
```

### macOS App

下载 `SkyNAS-x.x.x.zip`，解压后将 `SkyNAS.app` 拖到 `/Applications` 文件夹。

## Quick Start

```bash
# Run the server
skynas

# Or with custom port
skynas --port 8081

# Start in background
skynas start --background
```

Then scan the QR code with your iPhone camera and open the link.

## Configuration

Create `~/.config/skynas/config.toml`:

```toml
[server]
port = 8080

[storage]
base_path = "/Users/xxx/Pictures/iPhoneSync"

[sync]
enabled = true
command = "rclone sync ~/Pictures/iPhoneSync nas:Photos"
auto_sync = true

[heic_converter]
backend = "sips"  # or "image", "libheif"
generate_jpeg = true
```

## CLI Commands

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

## API Endpoints

- `GET /` - Web upload interface
- `GET /ws` - WebSocket for real-time progress
- `POST /api/upload` - Simple file upload
- `POST /api/upload/chunked/init` - Initialize chunked upload
- `POST /api/upload/chunked/chunk` - Upload chunk
- `POST /api/upload/chunked/complete/:id` - Complete chunked upload
- `GET /api/upload/chunked/status/:id` - Check upload status
- `GET /api/health` - Health check

## Architecture

- **HTTP Server**: Axum-based with multipart upload support
- **Database**: SQLite for metadata persistence
- **File Storage**: Organized by album in configurable base path
- **mDNS**: Zeroconf for service discovery
- **Notifications**: macOS native notifications via notify-rust

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Check code
cargo clippy
```

## License

MIT
