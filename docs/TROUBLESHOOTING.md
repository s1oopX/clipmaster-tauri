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

当前开发端口是 `5174`。

需要同步修改两个文件：

- `vite.config.js` 的 `server.port`
- `src-tauri/tauri.conf.json` 的 `build.devUrl`

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

## Tauri identifier 警告

当前构建会提示：

```text
The bundle identifier "com.clipmaster.app" ends with `.app`.
```

这是非阻断警告。后续建议改为：

```text
com.clipmaster.desktop
```

改动后需要留意应用数据目录可能变化。

## 图片不显示

检查：

- 数据库中的 `image_path` 是否类似 `images/2026-06/<file>.png`。
- 应用数据目录下图片文件是否存在。
- 前端是否调用 `convertImagePath`。
- 打包版是否仍能通过 Tauri asset URL 访问本地文件。

## 构建产物位置

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/msi/
src-tauri/target/release/bundle/nsis/
```
