# ClipMaster

[简体中文](./README.md) | [English](./README.en-US.md)

ClipMaster 是一款面向 Windows 桌面的轻量级剪贴板管理工具。它基于 Tauri 2、Rust、Svelte 5、Vite 8 和 SQLite 构建，专注于把文本、图片、截图和常用片段整理成可搜索、可收藏、可置顶、可恢复的本地历史记录。

项目当前处于早期发布阶段，适合个人效率工具、Tauri 桌面应用实践、Rust + Svelte 本地应用架构参考。

## 下载

最新 Windows 安装包会发布在 [GitHub Releases](https://github.com/s1oopX/clipmaster-tauri/releases/latest)。

当前版本提供：

- `ClipMaster_0.1.0_x64-setup.exe`：推荐的 Windows NSIS 安装包
- `ClipMaster_0.1.0_x64_en-US.msi`：Windows MSI 安装包

如果你从源码构建，安装包会生成在：

```text
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

## 功能亮点

- 监听并记录文本和图片剪贴板内容
- SQLite 本地持久化，不依赖云端服务
- 支持搜索、会话记录、收藏、置顶和删除
- 支持图片保存、缩略图预览和复制回剪贴板
- 支持区域截图、截图快捷键和图片置顶小窗
- 支持系统托盘：关闭主窗口后隐藏到托盘，可从托盘恢复或退出
- 支持自定义清理策略和图片文件生命周期管理
- 支持旧数据目录迁移，减少升级时历史记录丢失风险

## 技术栈

- Tauri 2
- Rust 2021
- Svelte 5
- Vite 8
- SQLite / rusqlite
- Vitest + Svelte Testing Library

## 快速开始

### 环境要求

- Windows 10/11
- Node.js 18 或更高版本
- npm
- Rust stable
- Visual Studio Build Tools with C++ workload

### 安装依赖

```powershell
npm install
```

### 启动开发模式

```powershell
npm run tauri:dev
```

开发端口默认为 `5174`。应用内“设置 / 端口”可以检查端口占用并切换端口；本机开发配置会写入被忽略的 `.clipmaster-dev.json`。

### 构建生产包

```powershell
npm run tauri:build
```

构建完成后会生成桌面程序、MSI 和 NSIS 安装包。Rust/Tauri 的 `src-tauri/target` 构建缓存可能较大，可以在不需要保留旧构建时删除；下次构建会自动重新生成。

## 质量验证

推荐在发布前执行：

```powershell
npm test
npm run build
cd src-tauri
cargo fmt --check
cargo check
cargo test
cd ..
npm run tauri:build
```

截至 2026-06-07，项目已经验证可以生成：

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/msi/ClipMaster_0.1.0_x64_en-US.msi
src-tauri/target/release/bundle/nsis/ClipMaster_0.1.0_x64-setup.exe
```

## 项目结构

```text
src/                 Svelte 前端
src/components/      前端组件
src/lib/             前端 API、配置和 UI 工具
src-tauri/src/       Rust 后端、数据库、剪贴板、托盘和命令
src-tauri/icons/     应用图标
docs/                架构、API、数据库、工作流和排障文档
```

## 文档

- [后续开发清单](./docs/NEXT_STEPS.md)
- [路线图](./docs/ROADMAP.md)
- [架构](./docs/ARCHITECTURE.md)
- [API](./docs/API.md)
- [数据库](./docs/DATABASE.md)
- [工作流](./docs/WORKFLOW.md)
- [排障](./docs/TROUBLESHOOTING.md)
- [变更记录](./CHANGELOG.md)

## 贡献

欢迎提交 issue、改进建议和 pull request。建议先阅读 [工作流](./docs/WORKFLOW.md) 和 [后续开发清单](./docs/NEXT_STEPS.md)，再根据功能范围补充必要的前端、后端或打包验证。

## 开源协议

本项目基于 [MIT License](./LICENSE) 开源。
