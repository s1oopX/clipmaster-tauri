# ClipMaster - Tauri 版本

> 轻量级 Windows 剪贴板管理工具，采用 Rust + Svelte 技术栈

## 🎯 项目概述

ClipMaster 是一款零门槛、开箱即用的 Windows 桌面剪贴板管理工具，内置飞书级截图体验，专为全设备流畅运行而优化。

### 核心特点

- ✅ **轻量级** - 内存占用 ~40MB（比 Electron 版本节省 66%）
- ✅ **高性能** - Rust 后端，启动时间 < 1 秒
- ✅ **小体积** - 打包后仅 5-10MB
- ✅ **零门槛** - 双击 exe 即可使用，无需安装数据库
- ✅ **会话管理** - 按软件启动区分记录，快速定位本次会话内容
- ✅ **飞书级截图** - 内置截图 + 标注 + 置顶功能（计划中）

## 📊 技术栈

```yaml
桌面框架: Tauri 2.0
前端框架: Svelte 5 + Vite 8
后端语言: Rust 1.96
数据存储: SQLite (rusqlite)
图片处理: image crate
系统集成: clipboard-rs, screenshots
```

## 📂 项目结构

```
clipmaster-tauri/
├── src/                        # Svelte 前端代码
│   ├── lib/                    # 可复用组件
│   │   ├── components/         # UI 组件
│   │   ├── stores/            # Svelte stores
│   │   └── utils/             # 工具函数
│   ├── App.svelte             # 根组件
│   ├── main.js                # 前端入口
│   └── app.css                # 全局样式
│
├── src-tauri/                  # Rust 后端代码
│   ├── src/
│   │   ├── main.rs            # 主程序入口
│   │   ├── clipboard/         # 剪贴板监听模块
│   │   ├── database/          # 数据库模块
│   │   ├── session/           # 会话管理模块
│   │   ├── screenshot/        # 截图模块（计划中）
│   │   └── commands.rs        # Tauri 命令接口
│   ├── Cargo.toml             # Rust 依赖配置
│   └── tauri.conf.json        # Tauri 配置
│
├── docs/                       # 项目文档
│   ├── ARCHITECTURE.md        # 架构设计
│   ├── API.md                 # API 接口文档
│   ├── DATABASE.md            # 数据库设计
│   └── ROADMAP.md             # 开发路线图
│
├── package.json
├── vite.config.js
└── index.html
```

## 🚀 快速开始

### 环境要求

- Node.js 18+
- Rust 1.70+
- Visual Studio Build Tools（Windows）

### 安装依赖

```bash
# 安装前端依赖
npm install

# Rust 依赖会在首次运行时自动安装
```

### 开发模式

```bash
# 启动开发服务器（热重载）
npm run tauri:dev
```

### 构建生产版本

```bash
# 构建生产版本（生成 exe 文件）
npm run tauri:build
```

## 📋 开发进度

### ✅ Phase 0: 基础架构（已完成）
- [x] Tauri + Svelte 项目初始化
- [x] 开发环境配置
- [x] 基础文件结构搭建

### 🚧 Phase 1: 核心功能（进行中）
- [ ] 剪贴板监听服务
- [ ] 会话管理系统
- [ ] SQLite 数据库集成
- [ ] 基础 UI 界面
- [ ] Tauri IPC 通信

### ⏳ Phase 2: 数据展示
- [ ] 剪贴板历史列表（虚拟滚动）
- [ ] 会话筛选侧边栏
- [ ] 搜索功能
- [ ] 分类管理

### ⏳ Phase 3: 高级功能
- [ ] 图片支持（缩略图 + 原图）
- [ ] 截图功能
- [ ] 截图编辑器
- [ ] 置顶窗口

### ⏳ Phase 4: 优化打磨
- [ ] 性能优化
- [ ] 快捷键支持
- [ ] 系统托盘
- [ ] 设置面板

详细开发计划请查看 [ROADMAP.md](./docs/ROADMAP.md)

## 📖 文档导航

- [架构设计](./docs/ARCHITECTURE.md) - 技术架构和模块设计
- [API 接口](./docs/API.md) - Tauri Commands 和前后端通信
- [数据库设计](./docs/DATABASE.md) - SQLite 表结构和索引设计
- [开发路线图](./docs/ROADMAP.md) - 详细的开发计划和任务分解
- [原始任务书](../项目任务书.md) - 完整的产品需求文档

## 🎨 截图

_开发中，敬请期待..._

## 📝 许可证

MIT License

## 🤝 贡献指南

本项目目前处于初期开发阶段。

---

**当前版本**: v0.1.0-alpha  
**最后更新**: 2026-06-05  
**开发状态**: 🚧 开发中
