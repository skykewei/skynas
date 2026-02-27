# 日志与 WebSocket 进度功能验证指南

本文档说明如何验证新实现的日志和 WebSocket 进度推送功能。

## 1. 启动 Server 并观察日志

### 使用 Debug 级别日志（开发调试）

```bash
cd /Users/sulin/RustroverProjects/skynas
RUST_LOG=debug cargo run
```

### 使用 Trace 级别日志（最详细）

```bash
RUST_LOG=trace cargo run
```

### 使用 Info 级别日志（生产环境）

```bash
RUST_LOG=info cargo run
```

## 2. 预期日志输出示例

### 上传初始化阶段

```
INFO skynas::server::upload: Upload session initialized upload_id=xxx filename=photo.jpg album=vacation total_size=5242880 total_chunks=10
DEBUG skynas::server::upload: Creating temp directory upload_id=xxx temp_path="/Users/xxx/Pictures/iPhoneSync/.temp/xxx"
DEBUG skynas::server::upload: Recording upload session in database upload_id=xxx
INFO skynas::server::upload: Upload session ready for chunks upload_id=xxx elapsed_ms=15
```

### 接收分片阶段

```
DEBUG skynas::server::upload: Receiving chunk upload_id=xxx chunk_index=0
DEBUG skynas::server::upload: Chunk received upload_id=xxx chunk_index=0 chunk_size=524288
DEBUG skynas::server::upload: Chunk saved to disk upload_id=xxx chunk_index=0 path="..."
INFO skynas::server::upload: Chunk processed upload_id=xxx chunk_index=0 total_chunks=10 received_chunks=1 percent=10 chunk_size=524288 elapsed_ms=85
```

### 合并完成阶段

```
INFO skynas::server::upload: Starting upload completion upload_id=xxx
INFO skynas::server::upload: Upload session retrieved, beginning merge upload_id=xxx filename=photo.jpg album=vacation total_chunks=10
DEBUG skynas::server::upload: Creating album directory upload_id=xxx album_path="/Users/xxx/Pictures/iPhoneSync/vacation"
DEBUG skynas::server::upload: Creating final file upload_id=xxx final_path="..."
INFO skynas::server::upload: All chunks merged upload_id=xxx total_chunks=10 elapsed_ms=45
DEBUG skynas::server::upload: Calculating file hash upload_id=xxx
DEBUG skynas::server::upload: File hash calculated upload_id=xxx hash="xxx..."
INFO skynas::server::upload: HEIC converted to JPEG upload_id=xxx original=photo.heic converted="..."
INFO skynas::server::upload: Photo saved to database upload_id=xxx
INFO skynas::server::upload: Upload completed successfully upload_id=xxx filename=photo.jpg album=vacation size_bytes=5242880 total_chunks=10 has_jpeg_variant=true total_elapsed_ms=2450
```

## 3. WebSocket 事件验证

### 使用浏览器控制台

1. 打开浏览器访问 `http://localhost:8080`
2. 打开开发者工具 (F12) -> Console
3. 粘贴以下代码连接 WebSocket：

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = function() {
    console.log('WebSocket connected');
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Received event:', data.type, data);
};

ws.onerror = function(error) {
    console.error('WebSocket error:', error);
};

ws.onclose = function() {
    console.log('WebSocket closed');
};
```

### 预期 WebSocket 事件序列

上传一个文件时，应该按顺序收到以下事件：

```javascript
// 1. 上传开始
{
    "type": "upload_started",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg",
    "album": "vacation",
    "total_bytes": 5242880,
    "total_chunks": 10
}

// 2. 分片接收（每个分片一次）
{
    "type": "chunk_received",
    "upload_id": "uuid-xxxx",
    "chunk_index": 0,
    "total_chunks": 10
}

// 3. 进度更新（每个分片一次）
{
    "type": "upload_progress",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg",
    "received_bytes": 524288,
    "total_bytes": 5242880,
    "percent": 10
}

// 4. 开始合并
{
    "type": "chunks_merging",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg"
}

// 5. 文件保存完成
{
    "type": "file_saved",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg",
    "path": "/Users/xxx/Pictures/iPhoneSync/vacation/photo.jpg",
    "size": 5242880
}

// 6. 开始 HEIC 转换（如果是 HEIC 文件）
{
    "type": "heic_converting",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg"
}

// 7. HEIC 转换完成
{
    "type": "heic_converted",
    "upload_id": "uuid-xxxx",
    "original": "photo.heic",
    "converted": "/Users/xxx/Pictures/iPhoneSync/vacation/photo.jpg",
    "success": true
}

// 8. 正在保存到数据库
{
    "type": "database_saving",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg"
}

// 9. 上传完成
{
    "type": "upload_complete",
    "upload_id": "uuid-xxxx",
    "filename": "photo.jpg",
    "album": "vacation",
    "size": 5242880
}
```

## 4. 使用 wscat 命令行工具测试

安装 wscat：
```bash
npm install -g wscat
```

连接 WebSocket：
```bash
wscat -c ws://localhost:8080/ws
```

然后上传文件，观察输出的 JSON 事件。

## 5. 故障排查

### 问题：看不到 Debug 日志

检查 `RUST_LOG` 环境变量是否设置正确：
```bash
RUST_LOG=debug cargo run
```

### 问题：WebSocket 没有收到事件

1. 确认 WebSocket 连接成功（看 onopen 回调）
2. 检查浏览器网络面板，确认 WebSocket 连接状态
3. 确认上传请求成功发送到了正确的 endpoint

### 问题：事件格式不正确

检查代码中的事件序列化函数 `serialize_event()` 是否正确处理了所有事件类型。

## 6. 单元测试

运行测试：
```bash
cd /Users/sulin/RustroverProjects/skynas
cargo test
```

测试包括：
- `websocket::tests::test_event_serialization` - 验证事件序列化
- `websocket::tests::test_event_channel` - 验证事件通道
- `server::upload::tests::test_percent_calculation` - 验证进度计算
- `server::upload::tests::test_ws_event_types` - 验证 WebSocket 事件类型
