# Troubleshooting

## `cargo` 或 `rustc` 找不到

重新打开终端，让 Rust 安装后的环境变量生效。

检查：

```powershell
rustc --version
cargo --version
```

如果仍然找不到，确认 `%USERPROFILE%\.cargo\bin` 在 `PATH` 中。

## `link.exe` 找不到

Windows 上构建 Tauri 需要 Visual Studio Build Tools。

安装时选择：

```text
Desktop development with C++
```

## 端口被占用

默认开发端口是 `5174`，可以在应用内“设置 / 端口”检查占用并切换。

端口变更会写入本机 `.clipmaster-dev.json`，该文件不会提交到 Git。开发模式请使用：

```powershell
npm run tauri:dev
```

这个脚本会把同一端口同步给 Vite 和 Tauri。

## `frontendDist` 不存在

`cargo check` 或 Tauri 宏可能报：

```text
The `frontendDist` configuration is set to `"../dist"` but this path doesn't exist
```

先运行：

```powershell
npm run build
```

生成 `dist/` 后再运行 Rust 检查。

## Vite 提示缺少 `esbuild`

Vite 8 的生产构建需要项目显式安装 `esbuild`。当前已经在 `devDependencies` 中声明。

修复命令：

```powershell
npm install --save-dev esbuild
```

## Tauri identifier 仍然提示 `.app`

当前 identifier 应为：

```text
com.clipmaster.desktop
```

如果构建仍提示：

```text
The bundle identifier "com.clipmaster.app" ends with `.app`.
```

检查 `src-tauri/tauri.conf.json` 是否被回退。升级后数据目录应位于 `%APPDATA%/com.clipmaster.desktop/`；首次启动会尝试从旧目录 `%APPDATA%/com.clipmaster.app/` 迁移。

## 冒烟测试写入了真实历史

打包版默认使用真实应用数据目录。需要隔离测试时，请在启动前设置：

```powershell
$env:CLIPMASTER_APP_DATA_DIR = "$env:TEMP\clipmaster-smoke-data"
src-tauri\target\release\clipmaster.exe
```

不要只临时修改 `APPDATA` 或 `LOCALAPPDATA`，Windows Known Folder 解析不一定会采用它们。

如果这次冒烟需要验证图片预览，不要把 `CLIPMASTER_APP_DATA_DIR` 指到任意临时目录。图片 URL 会经过 Tauri asset scope，隔离目录应放在 `%APPDATA%/com.clipmaster.desktop/Smoke-*` 这样的真实 app data 子目录下，测试后再删除该子目录。

## 图片不显示

检查：

- 数据库中的 `image_path` 是否类似 `images/2026-06-07/<file>.png`。
- 应用数据目录下图片文件是否存在。
- 前端是否调用 `convertImagePath`。
- 打包版是否仍能通过 Tauri asset URL 访问本地文件。
- 若使用 `CLIPMASTER_APP_DATA_DIR` 做隔离图片测试，确认隔离目录仍在 Tauri asset scope 允许的 app data 路径内。

## 构建产物位置

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/msi/
src-tauri/target/release/bundle/nsis/
```
