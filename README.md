<div align="center">

<img src="./src-tauri/icons/icon.png" alt="ClipMaster icon" width="96" height="96">

# ClipMaster

本地优先的 Windows 剪贴板管理器，用来找回、整理和复用你复制过的文字、图片与截图。

[English](./README.en-US.md) · [下载最新版](https://github.com/s1oopX/clipmaster-tauri/releases/latest) · [路线图](./docs/ROADMAP.md)

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-0078D4.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg)
![Rust](https://img.shields.io/badge/Rust-2021-B7410E.svg)
![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00.svg)

</div>

## 为什么做 ClipMaster

复制历史经常是工作流里最容易丢的部分：一段刚复制的命令、一张临时截图、一个反复要贴的片段，过几分钟就被新的剪贴板内容覆盖。ClipMaster 把这些内容保存在本机，让它们可以被搜索、收藏、置顶、复制回剪贴板，减少来回翻窗口和重复截图的时间。

它不是云同步剪贴板，也不是账号体系工具。当前目标很简单：在 Windows 桌面上提供一个轻量、清楚、可控的本地剪贴板历史。

## 下载

从 [GitHub Releases](https://github.com/s1oopX/clipmaster-tauri/releases/latest) 获取最新安装包。

| 文件 | 用途 |
| --- | --- |
| [`ClipMaster_0.1.0_x64-setup.exe`](https://github.com/s1oopX/clipmaster-tauri/releases/download/v0.1.0/ClipMaster_0.1.0_x64-setup.exe) | 推荐安装包，适合大多数 Windows 用户 |
| [`ClipMaster_0.1.0_x64_en-US.msi`](https://github.com/s1oopX/clipmaster-tauri/releases/download/v0.1.0/ClipMaster_0.1.0_x64_en-US.msi) | MSI 安装包，适合需要传统安装流程的场景 |

当前版本尚未做代码签名，Windows 可能出现 SmartScreen 提示。请从本仓库的 Release 页面下载，并在安装前确认发布者与文件来源。

## 核心能力

| 能力 | 说明 |
| --- | --- |
| 文本和图片历史 | 自动记录剪贴板里的文本与图片内容 |
| 搜索和筛选 | 快速查找历史记录、会话记录和常用片段 |
| 收藏和置顶 | 把高频内容固定在更容易找到的位置 |
| 图片工作流 | 保存图片、生成缩略图、预览图片并复制回剪贴板 |
| 截图辅助 | 支持区域截图、截图快捷键和图片置顶小窗 |
| 系统托盘 | 关闭窗口后隐藏到托盘，可随时恢复或退出 |
| 清理策略 | 支持按数量、时间和图片文件生命周期清理历史 |
| 数据迁移 | 内置旧数据目录迁移，减少升级时历史记录丢失风险 |

## 隐私和数据

ClipMaster 以本地存储为默认边界：

- 剪贴板历史保存在本机 SQLite 数据库。
- 图片和缩略图保存在本机应用数据目录。
- 当前版本不提供云同步、账号登录或远程遥测。
- 剪贴板可能包含密码、令牌和个人信息，建议定期清理敏感记录。

## 当前状态

ClipMaster 0.1.0 是早期可用版本，已经覆盖主要剪贴板、图片、截图、托盘和打包流程。它适合个人日常试用，也适合作为 Tauri 2 + Rust + Svelte 桌面应用的参考项目。

下一阶段会优先改进会话侧边栏、虚拟滚动、更多全局快捷键和前端状态拆分。详情见 [Roadmap](./docs/ROADMAP.md) 和 [Next Steps](./docs/NEXT_STEPS.md)。

## 技术栈

- Tauri 2
- Rust 2021
- Svelte 5
- Vite 8
- SQLite / rusqlite
- Vitest + Svelte Testing Library

## 本地开发

### 环境要求

- Windows 10/11
- Node.js 18 或更高版本
- npm
- Rust stable
- Visual Studio Build Tools，包含 C++ workload

### 安装依赖

```powershell
npm install
```

### 启动开发模式

```powershell
npm run tauri:dev
```

默认开发端口是 `5174`。应用内的设置面板可以检查端口占用并切换端口，本机配置写入 `.clipmaster-dev.json`，该文件不会提交到 Git。

### 构建安装包

```powershell
npm run tauri:build
```

构建产物会生成在：

```text
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

`src-tauri/target` 是 Rust/Tauri 构建缓存，体积可能较大。删除它不会影响源码，下次构建会重新生成，只是第一次编译会更慢。

## 常用命令

| 命令 | 说明 |
| --- | --- |
| `npm run tauri:dev` | 启动 Tauri 开发窗口 |
| `npm test` | 运行前端测试 |
| `npm run build` | 构建前端静态资源 |
| `npm run tauri:build` | 构建 Windows 桌面程序和安装包 |
| `cargo fmt --check` | 检查 Rust 格式 |
| `cargo check` | 检查 Rust 编译 |
| `cargo test` | 运行 Rust 单元测试 |

## 项目结构

```text
src/                 Svelte 前端入口
src/components/      前端组件
src/lib/             前端 API、配置和 UI 工具
src-tauri/src/       Rust 后端、数据库、剪贴板、托盘和命令
src-tauri/icons/     应用图标
docs/                架构、API、数据库、工作流和排障文档
```

## 文档

- [开发路线图](./docs/ROADMAP.md)
- [后续开发清单](./docs/NEXT_STEPS.md)
- [架构说明](./docs/ARCHITECTURE.md)
- [API 文档](./docs/API.md)
- [数据库说明](./docs/DATABASE.md)
- [开发工作流](./docs/WORKFLOW.md)
- [排障指南](./docs/TROUBLESHOOTING.md)
- [变更记录](./CHANGELOG.md)

## 贡献

欢迎提交 issue、建议和 pull request。改动前建议先阅读 [开发工作流](./docs/WORKFLOW.md)，并根据改动范围补充前端测试、Rust 测试或打包验证。

## License

ClipMaster 基于 [MIT License](./LICENSE) 开源。
