# Workflow

本文记录后续开发时建议遵守的验证流程。

## 常用命令

安装依赖：

```powershell
npm install
```

开发模式：

```powershell
npm run tauri:dev
```

前端构建：

```powershell
npm run build
```

前端测试：

```powershell
npm test
```

Rust 检查：

```powershell
cd src-tauri
cargo fmt --check
cargo check
cd ..
```

完整打包：

```powershell
npm run tauri:build
```

## 每次改动后的推荐检查

小改动：

```powershell
npm test
npm run build
cd src-tauri
cargo fmt --check
cargo check
cd ..
```

发版或打包前：

```powershell
npm run tauri:build
```

## 手动冒烟测试

- 启动应用。
- 复制一段普通文本，确认列表新增记录。
- 再复制同一段文本，确认不会短时间重复刷屏。
- 复制图片，确认图片保存并显示。
- 搜索文本，确认结果正确。
- 切换置顶和收藏，确认 UI 状态和排序正确。
- 删除记录，确认列表更新。
- 点击复制按钮，确认文本写回剪贴板。
- 重启应用，确认历史记录仍在。

## 提交前检查

- `git status` 中没有误加入 `dist/`、`target/`、`node_modules/`。
- 文档链接没有指向已删除文件。
- 新增 Tauri command 已同步更新：
  - Rust `generate_handler!`
  - `src/lib/api.js`
  - `docs/API.md`
- 数据库表结构变化已考虑迁移。

## 推荐提交粒度

- 一个功能一个提交。
- 一个修复一个提交。
- 纯文档整理单独提交。
- 不把构建产物提交进仓库。
