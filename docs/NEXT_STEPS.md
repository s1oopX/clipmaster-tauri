# 后续开发清单

这是继续开发时的入口文档。先处理上面的事项，再进入新功能。

## 立即处理

- 修正 Tauri identifier：建议从 `com.clipmaster.app` 改为 `com.clipmaster.desktop` 或 `com.clipmaster.clipmaster`。
- 运行打包版 `src-tauri/target/release/clipmaster.exe` 做冒烟测试。
- 验证 `%APPDATA%/com.clipmaster.app/clipboard.db` 和图片目录是否按预期生成。
- 确认删除记录时是否需要同时删除对应图片文件。
- 决定窗口关闭语义：退出应用，还是隐藏到托盘。

## 下一轮功能

- 图片复制回剪贴板。
- 会话筛选 UI。
- 虚拟滚动和分页加载。
- 前端组件拆分和 store 化。
- 系统托盘。
- 全局快捷键。

## 技术债

- `App.svelte` 已经承担太多 UI 和状态逻辑，需要拆分。
- API 文档和实际命令需要随新增 command 同步维护。
- 数据库没有迁移版本表，后续 schema 变更风险较高。
- 当前搜索是 `LIKE`，后续可升级 SQLite FTS5。
- 图片保存为 PNG，体积可能偏大，后续可考虑缩略图和 WebP。

## 建议顺序

1. 修 identifier 和冒烟测试。
2. 清理图片文件生命周期。
3. 拆前端组件。
4. 做会话侧边栏。
5. 做托盘和关闭行为。
6. 做快捷键和设置面板。
