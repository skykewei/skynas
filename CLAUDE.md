# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, and Lint Commands

```bash
# Build the project
cargo build --release

# Run the server locally
cargo run

# Run tests
cargo test --all-features --verbose

# Run a specific test
cargo test <test_name> --verbose

# Format code
cargo fmt --all

# Check formatting without modifying files
cargo fmt --all -- --check

# Run clippy (strict mode used in CI)
cargo clippy --all-targets --all-features -- -D warnings

# Install dependencies (macOS)
brew install libheif
```

## Project Architecture

SkyNAS is an async Rust web server for syncing photos from iPhone to Mac. It uses Axum for the HTTP layer and Tokio for async runtime.

### Module Structure

- `main.rs` - Entry point, CLI parsing with clap, and server initialization
- `server/mod.rs` - Axum router setup, middleware (CORS), route handlers
- `server/upload.rs` - Chunked upload implementation for large files
- `db/mod.rs` - SQLite operations via rusqlite (photos, chunks, sync operations)
- `models/mod.rs` - Data structures (Photo, UploadChunk, etc.)
- `config/mod.rs` - Configuration loading from `~/.config/skynas/config.toml` with defaults
- `websocket/mod.rs` - WebSocket handler for real-time upload progress
- `qr/mod.rs` - QR code generation for easy iPhone connection
- `mdns/mod.rs` - mDNS service publishing for local network discovery
- `converter/mod.rs` - HEIC to JPEG conversion with multiple backends (sips, image, libheif)
- `sync/mod.rs` - Cloud/NAS sync command execution
- `notify/mod.rs` - macOS native notifications via notify-rust
- `cli/mod.rs` - CLI command definitions

### Key Data Flow

1. Server starts on configured port (default 8080), displays QR code
2. iPhone connects via browser, uploads photos via chunked POST endpoints
3. Uploads saved to `~/.Pictures/iPhoneSync/<album>/` with metadata in SQLite
4. HEIC files optionally converted to JPEG
5. WebSocket sends real-time progress to connected clients
6. Optional: Command executed for cloud sync after upload

### Configuration

Default config location: `~/.config/skynas/config.toml`

Key sections:
- `[server]` - port, host, max_upload_size, chunk_size
- `[storage]` - base_path (where photos are saved)
- `[sync]` - enabled, command, auto_sync
- `[heic_converter]` - backend ("sips"/"image"/"libheif"), generate_jpeg, jpeg_quality
- `[features]` - mdns_enabled, websocket_enabled, notification_enabled

Environment variable override: `SKYNAS_PORT`

### API Endpoints

- `GET /` - Web UI
- `GET /ws` - WebSocket for progress updates
- `POST /api/upload` - Simple file upload
- `POST /api/upload/chunked/init` - Start chunked upload
- `POST /api/upload/chunked/chunk` - Upload chunk
- `POST /api/upload/chunked/complete/:id` - Complete upload
- `GET /api/upload/chunked/status/:id` - Check upload status
- `GET /api/health` - Health check

### CI/CD

GitHub Actions workflows in `.github/workflows/`:
- `ci.yml` - Runs fmt, clippy, tests, and build check on push/PR
- `coverage.yml` - Generates code coverage reports
- `release.yml` - Builds binaries for x86_64/arm64/universal and macOS app bundles on tag push

### Release Process

```bash
# Create and push a new version tag
git tag v0.1.1
git push origin v0.1.1
```

The release workflow automatically builds and uploads:
- `skynas-v{version}-{arch}.tar.gz` binaries
- `SkyNAS-v{version}.zip` (macOS app bundle)
- Updates the Homebrew tap at `skykewei/homebrew-skynas`
