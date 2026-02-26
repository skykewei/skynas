<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/icon.svg">
  <img src="../assets/icon.svg" alt="SkyNAS Logo" width="180" height="180">
</picture>

# SkyNAS Photo Sync

<h3>轻松将 iPhone 照片同步到 Mac</h3>

<p>
  <a href="https://github.com/skykewei/skynas/releases/latest">
    <img src="https://img.shields.io/github/v/release/skykewei/skynas?style=for-the-badge&logo=github&color=blue" alt="Release">
  </a>
  <a href="https://github.com/skykewei/skynas/actions">
    <img src="https://github.com/skykewei/skynas/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="../LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" alt="License">
  </a>
  <a href="https://github.com/skykewei/skynas/stargazers">
    <img src="https://img.shields.io/github/stars/skykewei/skynas?style=for-the-badge&color=yellow" alt="Stars">
  </a>
</p>

<p>
  <img src="https://img.shields.io/github/downloads/skykewei/skynas/total?style=for-the-badge&color=purple" alt="Downloads">
  <img src="https://img.shields.io/tokei/lines/github/skykewei/skynas?style=for-the-badge&color=orange" alt="代码行数">
  <img src="https://img.shields.io/github/last-commit/skykewei/skynas?style=for-the-badge&color=red" alt="最后提交">
</p>

<p>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey?style=for-the-badge&logo=apple" alt="Platform">
</p>

<p>
  <strong>🇨🇳 中文</strong> •
  <a href="../README.md">🇺🇸 English</a>
</p>

</div>

---

## ✨ 功能特性

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

---

## 📦 安装方式

### Homebrew（推荐）

```bash
brew tap skykewei/skynas
brew install skynas
```

### 二进制下载

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

### macOS App

下载 [Releases](https://github.com/skykewei/skynas/releases) 中的 `SkyNAS-x.x.x.zip`，解压后将 `SkyNAS.app` 拖到「应用程序」文件夹。

---

## 🚀 快速开始

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

---

## ⚙️ 配置说明

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
jpeg_quality = 85
```

---

## 🌐 iPhone 使用指南

1. 在 Mac 上启动 SkyNAS
2. 扫描终端显示的二维码
3. 在 iPhone Safari 中打开链接
4. 选择照片并上传
5. 照片将自动整理到 Mac 的「图片」文件夹，按相册分类存放

---

## 💻 命令行工具

```bash
skynas [选项] [命令]

命令：
  start      启动服务器
  stop       停止运行中的服务器
  status     显示服务器状态
  menu-bar   以菜单栏应用运行（仅限 macOS）
  help       显示帮助信息

选项：
  -c, --config <配置>  配置文件路径
  -p, --port <端口>    服务器端口
  -d, --daemon         后台守护进程模式
  -h, --help           显示帮助
```

---

## 🌐 API 接口

| 端点 | 方法 | 说明 |
|------|------|------|
| `/` | GET | 网页上传界面 |
| `/ws` | GET | WebSocket 实时更新 |
| `/api/upload` | POST | 简单文件上传 |
| `/api/upload/chunked/init` | POST | 初始化分片上传 |
| `/api/upload/chunked/chunk` | POST | 上传分片 |
| `/api/upload/chunked/complete/:id` | POST | 完成分片上传 |
| `/api/upload/chunked/status/:id` | GET | 查询上传状态 |
| `/api/health` | GET | 健康检查 |

---

## 🔧 开发构建

```bash
# 克隆仓库
git clone https://github.com/skykewei/skynas.git
cd skynas

# 构建发布版本
cargo build --release

# 运行测试
cargo test

# 代码检查
cargo clippy --all-targets --all-features -- -D warnings

# 格式化代码
cargo fmt --all
```

---

## 📊 技术栈

<p align="center">
  <img src="https://img.shields.io/badge/Rust-DEA584?style=for-the-badge&logo=rust&logoColor=black" alt="Rust">
  <img src="https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Tokio">
  <img src="https://img.shields.io/badge/Axum-7B68EE?style=for-the-badge&logo=rust&logoColor=white" alt="Axum">
  <img src="https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite">
  <img src="https://img.shields.io/badge/WebSocket-010101?style=for-the-badge&logo=socket.io&logoColor=white" alt="WebSocket">
</p>

- **HTTP 服务器**: Axum（异步 Web 框架）
- **数据库**: SQLite + rusqlite
- **运行时**: Tokio 异步运行时
- **mDNS**: Zeroconf 服务发现
- **通知**: notify-rust（macOS 原生通知）
- **图片处理**: image crate、sips、libheif

---

## 📄 开源协议

本项目采用 [MIT 协议](../LICENSE) 开源。

---

<div align="center">
  <p>
    用 ❤️ 构建 by <a href="https://github.com/skykewei">skykewei</a>
  </p>
  <p>
    <a href="https://github.com/skykewei/skynas/stargazers">
      <img src="https://img.shields.io/github/stars/skykewei/skynas?style=social" alt="Stars">
    </a>
  </p>
</div>
