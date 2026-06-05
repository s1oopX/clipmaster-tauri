# Changelog

## Unreleased

### Build

- 修复前端重复导出导致的 Vite 构建失败。
- 补充 `esbuild` devDependency。
- 将 Vite build target 调整为 `esnext`，匹配现代 Tauri WebView。
- 修复 Rust `search_items` 查询生命周期错误。
- 清理关闭窗口时的当前会话内存状态。
- 验证 `npm run tauri:build` 可以生成 exe、MSI 和 NSIS 安装包。

### Docs

- 删除一次性开发报告、过期搭建说明和重复构建说明。
- 保留面向后续维护的文档集合。
- 新增 `docs/NEXT_STEPS.md`。
- 更新 README、Roadmap、API、Database、Architecture、Workflow 和 Troubleshooting。

### Tests

- 新增 Vitest + Svelte Testing Library 测试环境。
- 为主界面增加搜索、筛选、空状态、记录列表和操作按钮的 UI 行为测试。

### UI

- 将主界面调整为桌面工具布局：左侧筛选栏、顶部搜索栏、紧凑历史列表。
- 使用 `@lucide/svelte` 替换大部分 emoji 图标。
- 为搜索框、筛选导航、图片预览和记录操作按钮补齐可访问标签。
- 压缩窄窗口顶部区域，优化图片记录空状态和列表项操作区密度。

## 0.1.0-alpha

### Added

- Tauri 2 + Svelte 5 + Vite 8 项目结构。
- Rust 剪贴板轮询服务。
- SQLite 数据库和会话管理。
- 剪贴板文本和图片记录。
- 搜索、删除、收藏、置顶。
- 图片文件保存和前端预览。
- 文本复制回剪贴板。
