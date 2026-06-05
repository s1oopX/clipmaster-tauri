# ClipMaster Tauri

ClipMaster 是一个 Windows 剪贴板管理工具，当前基于 Tauri 2、Rust、Svelte 5、Vite 8 和 SQLite 实现。

## 当前状态

截至 2026-06-05，项目可以完整构建：

```powershell
npm run build
cd src-tauri
cargo fmt --check
cargo check
cd ..
npm run tauri:build
```

已生成的生产产物：

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/msi/ClipMaster_0.1.0_x64_en-US.msi
src-tauri/target/release/bundle/nsis/ClipMaster_0.1.0_x64-setup.exe
```

Tauri 构建仍会提示 `com.clipmaster.app` 以 `.app` 结尾不推荐；这是后续需要处理的非阻断项。

## 已实现

- 文本和图片剪贴板轮询监听
- SQLite 持久化和会话记录
- 列表查询、会话查询、搜索、删除
- 收藏和置顶
- 图片保存到应用数据目录并在前端显示
- 文本复制回剪贴板
- Windows MSI 和 NSIS 安装包构建

## 待继续

优先看 [docs/NEXT_STEPS.md](./docs/NEXT_STEPS.md) 和 [docs/ROADMAP.md](./docs/ROADMAP.md)。

当前最值得继续做的是：

- 修正 Tauri identifier
- 做一轮真实运行冒烟测试
- 补图片复制功能
- 拆分 `src/App.svelte`，建立组件和 store
- 增加会话侧边栏、虚拟滚动、托盘和快捷键

## 开发环境

需要：

- Node.js 18 或更高版本
- npm
- Rust stable
- Visual Studio Build Tools with C++ workload

安装依赖：

```powershell
npm install
```

启动开发模式：

```powershell
npm run tauri:dev
```

开发端口固定为 `5174`，配置在 [vite.config.js](./vite.config.js) 和 [src-tauri/tauri.conf.json](./src-tauri/tauri.conf.json)。

## 项目结构

```text
src/                 Svelte 前端
src/lib/api.js       Tauri IPC API 封装
src-tauri/src/       Rust 后端
src-tauri/icons/     应用图标
docs/                后续维护文档
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
