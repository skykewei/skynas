# Server 端日志与 WebSocket 进度推送设计文档

## 问题描述

当前存在两个问题：
1. **日志缺失**: Server 端在处理图片上传时缺乏详细的日志输出，Debug 时难以追踪执行流程
2. **进度条不更新**: 前端上传图片时无法实时显示进度，也没有成功提示，原因是后端没有通过 WebSocket 发送进度事件

## 解决方案

### 1. 架构概述

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Client    │────▶│   Upload API  │────▶│   Logger    │
│  (Browser)  │◀────│  (Axum/Rust)  │◀────│  (tracing)  │
└─────────────┘     └──────────────┘     └─────────────┘
       ▲                     │
       │                     ▼
       │            ┌──────────────┐
       └────────────│  WebSocket   │
                   │ Event Sender │
                   └──────────────┘
```

### 2. 日志模块设计

#### 2.1 日志级别策略

| 级别 | 使用场景 | 示例 |
|------|---------|------|
| INFO | 用户可见的关键流程 | 上传开始/完成、文件保存路径 |
| DEBUG | 调试信息，详细流程 | 每个 chunk 接收、数据库操作 |
| TRACE | 最详细的数据 | 请求体内容、字节数据 |
| WARN | 非致命错误 | 文件已存在、转换失败但可继续 |
| ERROR | 需要处理的错误 | 数据库写入失败、磁盘满 |

#### 2.2 日志内容规范

每个上传请求应包含：
- `upload_id`: 唯一标识本次上传
- `filename`: 原始文件名
- `album`: 目标相册
- `chunk_index/total_chunks`: 分片信息
- `elapsed_ms`: 各阶段耗时

#### 2.3 日志输出示例

```
INFO  upload_started upload_id=xxx filename=photo.jpg album=vacation total_size=5.2MB total_chunks=10
DEBUG chunk_received upload_id=xxx chunk_index=3 size=524288 bytes elapsed_ms=150
INFO  upload_completed upload_id=xxx filename=photo.jpg album=vacation saved_path=/data/vacation/photo.jpg total_chunks=10 elapsed_ms=2300
DEBUG heic_conversion upload_id=xxx original=photo.heic converted=photo.jpg success=true
```

### 3. WebSocket 进度推送设计

#### 3.1 事件类型

在 `WsEvent` 基础上，补充更多细粒度事件：

```rust
pub enum WsEvent {
    // 现有事件
    UploadStarted { upload_id: String, filename: String, album: String, total_bytes: i64 },
    UploadProgress { upload_id: String, filename: String, received_bytes: i64, total_bytes: i64, percent: u8 },
    UploadComplete { upload_id: String, filename: String, album: String, size: i64 },
    UploadError { upload_id: String, filename: String, error: String },

    // 新增事件
    ChunkReceived { upload_id: String, chunk_index: i32, total_chunks: i32 },
    ChunksMerging { upload_id: String, filename: String },
    FileSaved { upload_id: String, filename: String, path: String },
    HeicConverting { upload_id: String, filename: String },
    DatabaseSaving { upload_id: String, filename: String },
    CloudSyncTriggered { upload_id: String },

    // 保持现有
    CloudSyncStarted,
    CloudSyncComplete { success: bool },
}
```

#### 3.2 推送时机

| 阶段 | 发送事件 | 说明 |
|------|---------|------|
| init_upload | UploadStarted | 告知前端开始上传 |
| upload_chunk (每片) | ChunkReceived + UploadProgress | 实时进度更新 |
| complete_upload (开始) | ChunksMerging | 开始合并分片 |
| complete_upload (文件保存后) | FileSaved | 告知保存位置 |
| HEIC 转换 | HeicConverting | 正在转换格式 |
| 数据库写入 | DatabaseSaving | 正在记录元数据 |
| 完成 | UploadComplete | 全部完成 |

### 4. 代码实现策略

#### 4.1 日志集成

1. 在 `main.rs` 初始化 `tracing_subscriber`
2. 在 `upload.rs` 中添加 `#[tracing::instrument]` 宏
3. 使用 `tracing::info!`, `tracing::debug!` 等宏记录日志

#### 4.2 WebSocket 事件发送

修改 `upload.rs` 中的函数，添加 `state.event_sender.send()` 调用：

```rust
// init_upload
state.event_sender.send(WsEvent::UploadStarted { ... }).ok();

// upload_chunk
state.event_sender.send(WsEvent::ChunkReceived { ... }).ok();
state.event_sender.send(WsEvent::UploadProgress { ... }).ok();

// complete_upload (各阶段)
state.event_sender.send(WsEvent::ChunksMerging { ... }).ok();
state.event_sender.send(WsEvent::FileSaved { ... }).ok();
// ... etc
```

### 5. 配置选项

在 `config.rs` 中添加日志配置：

```rust
pub struct LogConfig {
    pub level: String,      // "info", "debug", "trace"
    pub format: String,     // "pretty", "json", "compact"
    pub enable_file: bool,  // 是否写入日志文件
    pub file_path: Option<String>,
}
```

### 6. 测试策略

#### 6.1 单元测试

- 测试日志宏是否正确输出
- 测试 WebSocket 事件序列是否符合预期
- 测试错误情况下的日志记录

#### 6.2 集成测试

- 模拟完整上传流程，验证日志输出
- 使用 WebSocket 客户端接收事件，验证事件顺序和内容
- 测试并发上传时的日志和事件隔离

#### 6.3 手动验证

- 运行 server，观察控制台日志输出
- 使用浏览器上传图片，观察 WebSocket 事件和网络面板

### 7. 实现步骤

1. **初始化 tracing**: 在 main.rs 中添加初始化代码
2. **修改 WebSocket 事件**: 扩展 WsEvent 枚举
3. **添加日志到 upload.rs**: 使用 tracing 宏记录关键步骤
4. **添加 WebSocket 推送**: 在各阶段发送事件
5. **添加配置支持**: 可选的日志级别配置
6. **编写测试**: 验证日志和事件功能

## 风险评估

- **风险**: WebSocket 发送失败不应影响上传流程
  - **缓解**: 使用 `.ok()` 忽略发送错误

- **风险**: 大量日志可能影响性能
  - **缓解**: 使用适当的日志级别，生产环境用 INFO，Debug 用 DEBUG

- **风险**: 并发上传时日志混乱
  - **缓解**: 每个日志包含 upload_id，便于追踪
