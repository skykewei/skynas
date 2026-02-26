# GitHub Actions Workflows

## 工作流概览

| 工作流 | 触发条件 | 说明 |
|--------|---------|------|
| `ci.yml` | 所有 push 和 PR | 代码检查、测试、构建 |
| `coverage.yml` | push/PR 到 main/master | 代码覆盖率报告 |
| `release.yml` | 推送 v* 标签 | 构建发布产物 |

## 设置说明

### 1. Codecov 集成（可选）

如需代码覆盖率报告：

1. 访问 https://codecov.io
2. 使用 GitHub 登录并授权 `skykewei/skynas` 仓库
3. 获取 Repository Upload Token
4. 在仓库设置中添加 Secret：`CODECOV_TOKEN`

### 2. 发布新版本

```bash
# 1. 更新版本号（ Cargo.toml ）
vim Cargo.toml  # 修改 version = "x.x.x"

# 2. 提交更改
git add Cargo.toml
 git commit -m "chore: bump version to x.x.x"

# 3. 打标签并推送
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

推送标签后，`release.yml` 会自动：
- 创建 GitHub Release
- 构建 x86_64、arm64 和 universal 二进制文件
- 打包 macOS .app 应用
- 生成 Homebrew Formula

### 3. 产物说明

每次发布会生成以下产物：

| 产物 | 说明 | 安装方式 |
|------|------|---------|
| `skynas-vX.X.X-universal.tar.gz` | Universal 二进制（推荐） | 解压后直接运行 |
| `skynas-vX.X.X-x86_64.tar.gz` | Intel Mac 专用 | 解压后直接运行 |
| `skynas-vX.X.X-arm64.tar.gz` | Apple Silicon 专用 | 解压后直接运行 |
| `SkyNAS-vX.X.X.zip` | macOS 应用包（.app） | 拖到 /Applications |
| `skynas.rb` | Homebrew 公式 | `brew install skynas` |

## 注意事项

### 代码签名

目前 macOS App 使用临时签名（ad-hoc）。用户在首次打开时会看到安全警告，需要：
- 右键点击应用 → 打开 → 确认运行
- 或在系统偏好设置中允许

如需正式签名，需要：
1. Apple Developer 账号（$99/年）
2. 生成 Developer ID 证书
3. 配置 GitHub Secrets：
   - `APPLE_CERTIFICATE`（base64 编码的 .p12）
   - `APPLE_CERTIFICATE_PASSWORD`
   - `APPLE_ID`
   - `APPLE_APP_SPECIFIC_PASSWORD`

### 依赖项

项目在 CI 中自动安装以下依赖：
- `libheif`（HEIC 图片处理）

### Homebrew 发布

Homebrew 需要额外的 tap 仓库。推荐的发布流程：

1. 创建 `skykewei/homebrew-skynas` 仓库
2. 每次发布后，手动更新 formula 文件
3. 用户安装：`brew install skykewei/skynas/skynas`

自动化方案需要额外的 PAT Token 权限配置。
