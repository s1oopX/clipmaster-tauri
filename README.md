<div align="center">

<img src="./src-tauri/icons/icon.png" alt="ClipMaster icon" width="96" height="96">

# ClipMaster

本地优先的 Windows 剪贴板管理器，用于记录、检索、复用文本、图片与截图。

[English](./README.en-US.md) · [下载最新版](https://github.com/s1oopX/clipmaster-tauri/releases/latest) · [路线图](./docs/ROADMAP.md) · [安全策略](./SECURITY.md)

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-0078D4.svg)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB.svg)
![Rust](https://img.shields.io/badge/Rust-2021-B7410E.svg)
![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00.svg)

</div>

## 项目简介

ClipMaster 是一款面向 Windows 桌面的本地剪贴板工具。它会在本机保存文本、图片和截图历史，提供搜索、收藏、置顶、贴图、标注和清理能力，帮助用户找回临时复制内容并减少重复截图、重复复制和来回切换窗口的时间。

本项目不提供云同步、账号系统或远程遥测。数据默认保存在本机应用数据目录中，适合希望保留本地控制权的个人工作流、开发调试、文档编写、客服沟通和轻量图片复用场景。

## 当前状态

当前版本线为 `0.1.1`。项目已经覆盖日常可用的剪贴板历史、图片历史、区域截图、冻结屏幕选区、基础标注、桌面贴图、系统托盘、设置、清理策略、数据迁移和 Windows 打包流程。

仍需继续完善的方向包括虚拟滚动、更多全局快捷键、OCR、滚动截图、备份恢复、自动更新和代码签名。详情见 [Roadmap](./docs/ROADMAP.md)。

## 下载与安装

正式安装包发布在 [GitHub Releases](https://github.com/s1oopX/clipmaster-tauri/releases/latest)。

| 文件 | 适用场景 |
| --- | --- |
| `ClipMaster_0.1.1_x64-setup.exe` | 推荐安装包，适合大多数 Windows 用户 |
| `ClipMaster_0.1.1_x64_en-US.msi` | MSI 安装包，适合传统部署、企业环境或需要 MSI 的场景 |
| `SHA256SUMS.txt` | 发布文件校验信息 |

当前构建尚未进行代码签名，Windows SmartScreen 可能出现提示。请仅从本仓库 Release 页面下载，并根据 Release 附带的 SHA256 文件校验安装包。

## 功能概览

| 模块 | 能力 |
| --- | --- |
| 剪贴板历史 | 自动记录文本和图片剪贴板内容，支持复制回剪贴板 |
| 搜索与筛选 | 支持内容搜索、类型筛选、日期筛选和会话记录查看 |
| 收藏与置顶 | 将重要记录固定在更容易找到的位置 |
| 图片工作流 | 保存原图、生成缩略图、预览图片、复制图片、钉到桌面 |
| 截图工作流 | 区域截图、冻结屏幕选区、自动复制、保存历史、重新框选、矩形/箭头/画笔标注 |
| 桌面贴图 | 将图片以置顶小窗方式贴到桌面，便于对照和复用 |
| 系统托盘 | 关闭主窗口后隐藏到托盘，支持恢复和退出 |
| 设置管理 | 支持截图快捷键、截图延迟、保留数量、时区、语言、开机自启动等配置 |
| 数据清理 | 支持按数量、天数、图片生命周期清理，以及一键清空全部历史 |
| 数据迁移 | 内置旧数据目录迁移和数据库 schema migration |

## 截图体验

ClipMaster 的截图功能面向“截完马上用”的工作流：

- 启动截图时先捕获当前屏幕快照，并在冻结画面上进行框选。
- 选区支持拖动移动、8 个控制点缩放和方向键 1px 微调。
- 确认后自动保存到历史记录，并写入系统剪贴板，可直接 `Ctrl+V` 粘贴。
- 支持矩形、箭头、画笔三类基础标注，最终输出会包含标注内容。
- 支持重新框选和截图后直接钉到桌面。

## 隐私与数据

ClipMaster 以本地存储为默认边界：

- 剪贴板历史保存在本机 SQLite 数据库。
- 图片、缩略图和截图文件保存在本机应用数据目录。
- 当前版本不上传剪贴板内容，不提供云同步，不包含远程遥测。
- 剪贴板可能包含密码、令牌、客户资料或截图敏感信息。复制敏感内容前建议暂停监听，或在高级设置中清空全部历史。

默认数据目录为：

```text
%APPDATA%/com.clipmaster.desktop/
```

更多说明见 [Privacy](./docs/PRIVACY.md) 和 [Database](./docs/DATABASE.md)。

## 技术栈

- Tauri 2
- Rust 2021
- Svelte 5
- Vite 8
- SQLite / rusqlite
- Vitest / Svelte Testing Library
- screenshots / arboard / image

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

默认开发端口为 `5174`。应用内设置面板可以检查端口占用并切换端口，本地配置写入 `.clipmaster-dev.json`，该文件不会提交到 Git。

### 构建 Windows 程序与安装包

```powershell
npm run tauri:build
```

构建产物位于：

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

`node_modules`、`dist` 和 `src-tauri/target` 都是可重新生成的开发或构建产物，不建议提交到仓库。

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
src/                 Svelte 前端入口和页面逻辑
src/components/      前端组件
src/lib/             前端 API、配置和 UI 工具
src-tauri/src/       Rust 后端、数据库、剪贴板、托盘、命令和设置
src-tauri/icons/     应用图标
docs/                架构、API、数据库、工作流、路线图和排障文档
public/              静态资源
scripts/             本地开发脚本
```

## 文档

- [路线图](./docs/ROADMAP.md)
- [后续开发清单](./docs/NEXT_STEPS.md)
- [架构说明](./docs/ARCHITECTURE.md)
- [API 文档](./docs/API.md)
- [数据库说明](./docs/DATABASE.md)
- [开发工作流](./docs/WORKFLOW.md)
- [隐私与数据](./docs/PRIVACY.md)
- [用户 FAQ](./docs/FAQ.md)
- [安全策略](./SECURITY.md)
- [排障指南](./docs/TROUBLESHOOTING.md)
- [变更记录](./CHANGELOG.md)

## 贡献

欢迎提交 issue、改进建议和 pull request。提交改动前建议先阅读 [开发工作流](./docs/WORKFLOW.md)，并根据改动范围补充前端测试、Rust 测试或打包验证。

如果问题涉及剪贴板内容、截图、令牌、密码或其他敏感数据，请不要在公开 issue 中粘贴真实内容。安全问题请参考 [Security Policy](./SECURITY.md)。

## 许可证

ClipMaster 基于 [MIT License](./LICENSE) 开源。
