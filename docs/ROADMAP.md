# ClipMaster Roadmap

本文只保留后续开发需要的路线图，历史过程报告不再拆成多份文档。

## 当前基线

日期：2026-06-07

当前公开版本线：`0.1.1`

当前基线已经可以完成：

- `npm test`
- `npm run build`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `npm run tauri:build`

核心能力：

- 剪贴板文本和图片监听
- SQLite 存储
- 会话管理
- 搜索、删除、收藏、置顶
- 图片保存、缩略图、预览和文件清理
- 文本和图片复制回剪贴板
- 区域截图、截图快捷键和图片置顶小窗
- 设置面板、开发端口切换、自定义清理
- `com.clipmaster.desktop` identifier 和旧数据目录迁移
- `schema_migrations` 数据库迁移版本表
- 打包版受控冒烟：临时数据目录、旧目录迁移、文本剪贴板捕获
- 系统托盘：关闭隐藏、托盘显示和退出
- 自动化 UI 冒烟：图片预览/复制、搜索、收藏、置顶、删除、重启加载
- 打包版重启持久化烟测
- 打包版真实 WebView 图片预览烟测
- Windows exe、MSI、NSIS 安装包构建
- GitHub 仓库公开、正式项目图标和双语 README
- Release 安装包 SHA256 校验文件
- 隐私与 FAQ 文档
- 主界面暂停/恢复剪贴板监听入口
- 关于面板显示实际应用数据目录

## P0：稳定当前 MVP

- 发版前按 `docs/WORKFLOW.md` 做人工窗口走查。
- 后续表结构变化必须通过 `schema_migrations` 记录和测试。
- 发布安装包必须附带 SHA256 校验文件。

## P1：补齐日常使用体验

- 开机自启动。
- 全局快捷键呼出主窗口和聚焦搜索。
- 会话侧边栏：本次会话、上次会话、今天、全部历史。
- 列表虚拟滚动，避免数据多时 UI 卡顿。
- 前端组件拆分：`ClipboardList`、`ClipboardItem`、`SearchBar`、`SessionSidebar`。
- 引入 Svelte store，避免所有状态继续堆在 `App.svelte`。
- 搜索输入防抖和搜索结果高亮。

## P2：桌面集成

- 系统托盘增强：托盘状态提示、更多菜单项。
- 设置面板补充：开机启动、更多窗口行为设置。
- 周期后台清理任务：复用现有按数量和天数清理普通记录的能力。
- 应用黑名单和临时隐私模式。

## P3：高级能力

- 截图标注编辑器。
- OCR 和链接/代码自动分类。
- 导入导出和备份恢复。
- Tauri updater 和应用内检查更新。
- Windows 代码签名。

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
