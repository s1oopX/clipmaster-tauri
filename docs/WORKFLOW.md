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

为了避免污染真实历史记录，打包版冒烟可以用临时应用数据目录：

```powershell
$env:CLIPMASTER_APP_DATA_DIR = "$env:TEMP\clipmaster-smoke-data"
src-tauri\target\release\clipmaster.exe
```

`CLIPMASTER_APP_DATA_DIR` 会覆盖数据库、设置和图片目录。不要只依赖临时修改 `APPDATA`，Tauri/Windows Known Folder 不一定会使用该环境变量。

如果要验证打包版图片预览，隔离目录应放在真实 app data 目录下的子目录中，确保路径仍位于 Tauri asset scope 内：

```powershell
$smokeRoot = Join-Path $env:APPDATA "com.clipmaster.desktop\Smoke-$([guid]::NewGuid().ToString('N'))"
$env:CLIPMASTER_APP_DATA_DIR = $smokeRoot
src-tauri\target\release\clipmaster.exe
```

- 启动应用。
- 复制一段普通文本，确认列表新增记录。
- 再复制同一段文本，确认不会短时间重复刷屏。
- 复制图片，确认图片保存并显示。
- 搜索文本，确认结果正确。
- 切换置顶和收藏，确认 UI 状态和排序正确。
- 删除记录，确认列表更新。
- 点击复制按钮，确认文本写回剪贴板。
- 关闭主窗口，确认应用仍在托盘运行；从托盘菜单显示窗口。
- 重启应用，确认历史记录仍在。

验证完成后关闭应用，清理临时目录，并恢复当前终端里的 `CLIPMASTER_APP_DATA_DIR`。

## 提交前检查

- `git status` 中没有误加入 `dist/`、`target/`、`node_modules/`。
- 文档链接没有指向已删除文件。
- 新增 Tauri command 已同步更新：
  - Rust `generate_handler!`
  - `src/lib/api.js`
  - `docs/API.md`
- 数据库表结构变化已追加 `schema_migrations` 版本和旧库升级测试。

## 发版文档同步

每次发版或更新公开版本线后，同步检查这些文件，避免版本基线再次漂移：

- `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` / `src/lib/app-config.js`：应用版本号和界面展示版本一致。
- `CHANGELOG.md`：新增或更新对应版本段，说明用户可见变化。
- `docs/ROADMAP.md`：更新“当前基线”的复核日期、当前公开版本线和必要的核心能力清单。
- `docs/RELEASES.md`：确认最新公开版本、历史本地产物目录和发布前复核命令。
- `docs/NEXT_STEPS.md`：如果后续优先级变化，更新下一轮功能顺序。

复核命令：

```powershell
Select-String -Path package.json,src-tauri/tauri.conf.json,src-tauri/Cargo.toml,src/lib/app-config.js -Pattern '"version"|version =|appVersion'
Select-String -Path CHANGELOG.md,docs/ROADMAP.md,docs/RELEASES.md -Pattern '^## |Latest tagged version|当前公开版本线|当前基线'
```

## 推荐提交粒度

- 一个功能一个提交。
- 一个修复一个提交。
- 纯文档整理单独提交。
- 不把构建产物提交进仓库。
