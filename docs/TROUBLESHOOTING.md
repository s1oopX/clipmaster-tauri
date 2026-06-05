# ClipMaster 启动问题排查指南

## ❌ 问题：cargo 命令找不到

### 原因

Rust 环境变量在当前 PowerShell/命令提示符会话中未生效。

---

## ✅ 解决方案

### 方案 1：重新打开终端（推荐）

1. **关闭所有** PowerShell 和命令提示符窗口
2. 重新打开一个**新的**命令提示符或 PowerShell
3. 双击运行 `start.bat`

**原理**：环境变量只在新会话中生效

---

### 方案 2：使用更新后的启动脚本

我已经更新了 `start.bat`，它会自动加载 Cargo 路径。

直接双击 `start.bat` 即可。

---

### 方案 3：手动验证 Rust 环境

打开**新的** PowerShell 窗口，运行：

```powershell
cargo --version
rustc --version
```

如果显示版本号（1.96.0），说明环境正常。

然后运行：

```powershell
cd D:\Agent\clipmaster-tauri
npm run tauri:dev
```

---

## 🔍 如果还是失败

### 检查 Rust 是否真的安装了

```powershell
# 检查 Cargo 可执行文件是否存在
Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe"
```

如果返回 `False`，说明 Rust 未正确安装。

### 重新安装 Rust

1. 访问：https://rustup.rs/
2. 下载并运行 `rustup-init.exe`
3. 选择默认安装选项
4. 安装完成后**重启电脑**
5. 重新运行 `start.bat`

---

## 📝 首次启动注意事项

### 编译时间

- **首次启动**：需要编译 Rust 代码，约 **5-10 分钟**
- **后续启动**：< 5 秒

### 编译过程

你会看到类似这样的输出：

```
Compiling rusqlite v0.32.0
Compiling tokio v1.38.0
Compiling tauri v2.0.0
...
Finished dev [unoptimized + debuginfo] target(s) in 8m 23s
```

这是正常的，请耐心等待。

---

## 🚀 成功启动的标志

当看到以下输出时，说明启动成功：

```
Session started: session_xxxxx
ClipMaster started successfully!

  VITE v8.0.16  ready in 938 ms

  ➜  Local:   http://localhost:5173/
```

此时会自动打开应用窗口。

---

## 💡 建议的启动流程

### 第一次启动（需要编译）

1. 关闭所有终端窗口
2. 打开**新的**命令提示符（以管理员身份）
3. 双击 `D:\Agent\clipmaster-tauri\start.bat`
4. 等待 5-10 分钟编译
5. 应用自动打开

### 之后启动（快速）

直接双击 `start.bat` 即可，约 5 秒启动。

---

## 🐛 常见错误

### 错误 1：`program not found`

**原因**：环境变量未生效

**解决**：关闭终端，打开新终端

---

### 错误 2：`linker 'link.exe' not found`

**原因**：缺少 Visual Studio Build Tools

**解决**：

1. 下载 Visual Studio Build Tools: https://visualstudio.microsoft.com/downloads/
2. 安装时选择 "Desktop development with C++"
3. 重新运行 `start.bat`

---

### 错误 3：编译超时

**原因**：网络问题，下载依赖慢

**解决**：

配置 Cargo 国内镜像：

```powershell
# 创建配置文件
mkdir $env:USERPROFILE\.cargo
notepad $env:USERPROFILE\.cargo\config.toml
```

添加内容：

```toml
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
```

保存后重新运行。

---

## 📞 需要帮助？

如果上述方案都无法解决，请提供：

1. PowerShell 中运行 `cargo --version` 的输出
2. `start.bat` 的完整错误信息
3. Windows 版本

---

**更新时间**：2026-06-05  
**适用版本**：ClipMaster v0.1.0-alpha
