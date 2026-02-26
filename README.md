<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/icon.svg">
  <img src="assets/icon.svg" alt="SkyNAS Logo" width="180" height="180">
</picture>

# SkyNAS Photo Sync

<h3>
  <span lang="en">Sync photos from iPhone to Mac with ease</span>
  <br/>
  <span lang="zh">轻松将 iPhone 照片同步到 Mac</span>
</h3>

<p>
  <a href="https://github.com/skykewei/skynas/releases/latest">
    <img src="https://img.shields.io/github/v/release/skykewei/skynas?style=for-the-badge&logo=github&color=blue" alt="Release">
  </a>
  <a href="https://github.com/skykewei/skynas/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/skykewei/skynas/ci.yml?style=for-the-badge&logo=github-actions&logoColor=white&label=CI" alt="CI">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/github/license/skykewei/skynas?style=for-the-badge&color=green" alt="License">
  </a>
</p>

<p>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/github/languages/code-size/skykewei/skynas?style=for-the-badge&color=orange" alt="Code Size">
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey?style=for-the-badge&logo=apple" alt="Platform">
</p>

<p>
  <a href="#-english">English</a> •
  <a href="#-中文">中文</a> •
  <a href="#-installation">Install</a> •
  <a href="#-usage">Usage</a> •
  <a href="#-features">Features</a>
</p>

</div>

---

## 🇺🇸 English

### ✨ Features

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

### 📦 Installation

#### Homebrew (Recommended)

```bash
brew tap skykewei/skynas
brew install skynas
```

#### Binary Download

Download from [Releases](https://github.com/skykewei/skynas/releases):

```bash
# Intel Mac
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-x86_64.tar.gz

# Apple Silicon
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-arm64.tar.gz

# Universal Binary
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-universal.tar.gz

# Install
tar -xzf skynas.tar.gz
sudo mv skynas-*/skynas /usr/local/bin/
```

#### macOS App

Download `SkyNAS-x.x.x.zip` from [Releases](https://github.com/skykewei/skynas/releases), extract and drag `SkyNAS.app` to `/Applications`.

### 🚀 Quick Start

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

### ⚙️ Configuration

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
```

### 🌐 Using with iPhone

1. Start SkyNAS on your Mac
2. Scan the QR code displayed in terminal
3. Open the link on iPhone Safari
4. Select photos and upload
5. Photos appear organized in Mac's Pictures folder

---

## 🇨🇳 中文

### ✨ 功能特性

- **📱 无需安装 App** - iPhone 通过浏览器直接访问
- **📷 二维码连接** - 扫码即刻连接
- **📁 相册自动整理** - 按相册自动分类
- **🔄 断点续传** - 大文件传输不怕中断
- **🔍 重复检测** - SHA256 哈希去重
- **🖼️ HEIC 转 JPEG** - 多种转换后端支持
- **☁️ 自动云同步** - 上传后自动同步到 NAS/云端
- **📊 实时进度** - WebSocket 实时更新
- **🔎 mDNS 自动发现** - 局域网内自动发现 Mac
- **🔔 原生通知** - macOS 系统级通知

### 📦 安装方式

#### Homebrew（推荐）

```bash
brew tap skykewei/skynas
brew install skynas
```

#### 二进制下载

从 [Releases](https://github.com/skykewei/skynas/releases) 下载：

```bash
# Intel Mac
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-x86_64.tar.gz

# Apple Silicon M1/M2/M3
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-arm64.tar.gz

# 通用二进制（推荐）
curl -L -o skynas.tar.gz https://github.com/skykewei/skynas/releases/latest/download/skynas-latest-universal.tar.gz

# 安装
tar -xzf skynas.tar.gz
sudo mv skynas-*/skynas /usr/local/bin/
```

#### macOS App

下载 [Releases](https://github.com/skykewei/skynas/releases) 中的 `SkyNAS-x.x.x.zip`，解压后将 `SkyNAS.app` 拖到「应用程序」文件夹。

### 🚀 快速开始

```bash
# 启动服务（默认端口 8080）
skynas

# 指定端口
skynas --port 8081

# 后台运行
skynas start --background

# 查看状态
skynas status

# 停止服务
skynas stop
```

### ⚙️ 配置说明

创建配置文件 `~/.config/skynas/config.toml`：

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
backend = "sips"  # 可选: sips, image, libheif
generate_jpeg = true
```

### 🌐 iPhone 使用指南

1. 在 Mac 上启动 SkyNAS
2. 扫描终端显示的二维码
3. 在 iPhone Safari 中打开链接
4. 选择照片并上传
5. 照片将自动整理到 Mac 的「图片」文件夹

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

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Web upload interface |
| `/ws` | GET | WebSocket for real-time updates |
| `/api/upload` | POST | Simple file upload |
| `/api/upload/chunked/init` | POST | Initialize chunked upload |
| `/api/upload/chunked/chunk` | POST | Upload chunk |
| `/api/upload/chunked/complete/:id` | POST | Complete chunked upload |
| `/api/health` | GET | Health check |

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
