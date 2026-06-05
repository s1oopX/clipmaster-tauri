# ClipMaster Roadmap

本文只保留后续开发需要的路线图，历史过程报告不再拆成多份文档。

## 当前基线

日期：2026-06-05

当前基线已经可以完成：

- `npm run build`
- `cargo fmt --check`
- `cargo check`
- `npm run tauri:build`

核心能力：

- 剪贴板文本和图片监听
- SQLite 存储
- 会话管理
- 搜索、删除、收藏、置顶
- 图片保存和预览
- 文本复制回剪贴板
- Windows exe、MSI、NSIS 安装包构建

## P0：稳定当前 MVP

- 修正 `src-tauri/tauri.conf.json` 中的 `identifier`，避免 `.app` 结尾。
- 做真实运行冒烟测试：文本复制、图片复制、搜索、置顶、收藏、删除、重启后历史记录。
- 确认图片预览在打包版本中仍能通过 `convertFileSrc` 正常显示。
- 梳理关闭行为：直接退出还是最小化到托盘，需要产品决策。
- 给数据库增加迁移版本记录，避免后续表结构变化破坏旧数据。

## P1：补齐日常使用体验

- 图片复制回剪贴板。
- 会话侧边栏：本次会话、上次会话、今天、全部历史。
- 列表虚拟滚动，避免数据多时 UI 卡顿。
- 前端组件拆分：`ClipboardList`、`ClipboardItem`、`SearchBar`、`SessionSidebar`。
- 引入 Svelte store，避免所有状态继续堆在 `App.svelte`。
- 搜索输入防抖和搜索结果高亮。

## P2：桌面集成

- 系统托盘：显示、隐藏、退出。
- 全局快捷键：显示窗口、聚焦搜索、删除选中项。
- 设置面板：最大记录数、自动清理、开机启动、快捷键。
- 自动清理：按数量和天数清理普通记录，保留置顶和收藏。

## P3：高级能力

- 截图捕获。
- 截图标注编辑器。
- OCR 和链接/代码自动分类。
- 导入导出和备份恢复。

## 发布前检查

每次准备发版前至少执行：

```powershell
npm run build
cd src-tauri
cargo fmt --check
cargo check
cd ..
npm run tauri:build
```

并手动验证：

- 新复制内容能进入列表。
- 重复内容不会频繁刷屏。
- 搜索、删除、收藏、置顶可用。
- 打包版能启动，图片能显示。
