# ClipMaster Tauri 项目手动搭建指南

## 🎯 当前进度

✅ Rust 已安装：1.96.0  
✅ Cargo 已安装：1.96.0  
✅ 项目目录已创建：D:\Agent\clipmaster-tauri  
✅ Tauri 依赖已安装  
✅ Svelte 依赖已安装  
⏳ 正在初始化 Tauri 配置

---

## 📋 手动搭建步骤

### 已完成步骤

```bash
# 1. 创建项目目录
cd D:\Agent
mkdir clipmaster-tauri
cd clipmaster-tauri
npm init -y

# 2. 安装 Tauri 依赖
npm install @tauri-apps/cli @tauri-apps/api --save-dev

# 3. 安装 Svelte 依赖
npm install svelte @sveltejs/vite-plugin-svelte vite --save-dev
```

### 待完成步骤

```bash
# 4. 初始化 Tauri
npx tauri init \
  --app-name ClipMaster \
  --window-title ClipMaster \
  --frontend-dist ../dist \
  --dev-url http://localhost:5173 \
  --before-dev-command "npm run dev" \
  --before-build-command "npm run build"

# 5. 创建前端文件结构
mkdir src
# 创建 src/App.svelte
# 创建 src/main.js
# 创建 index.html

# 6. 配置 Vite
# 创建 vite.config.js

# 7. 更新 package.json
# 添加 dev 和 build 脚本

# 8. 测试运行
npm run tauri dev
```

---

## 📂 完整项目结构

```
clipmaster-tauri/
├── src/                    # Svelte 前端代码
│   ├── App.svelte
│   ├── main.js
│   └── lib/
├── src-tauri/              # Tauri/Rust 后端代码
│   ├── src/
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── vite.config.js
└── package.json
```

---

## 🔧 配置文件模板

### vite.config.js
```javascript
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
```

### package.json (scripts)
```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  }
}
```

### src/App.svelte
```svelte
<script>
  import { onMount } from 'svelte';

  let greeting = 'Welcome to ClipMaster!';

  onMount(async () => {
    // 初始化代码
  });
</script>

<main>
  <h1>{greeting}</h1>
</main>

<style>
  main {
    text-align: center;
    padding: 1em;
  }
</style>
```

### src/main.js
```javascript
import './app.css';
import App from './App.svelte';

const app = new App({
  target: document.getElementById('app'),
});

export default app;
```

### index.html
```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>ClipMaster</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>
```

---

## 🚀 启动命令

```bash
# 开发模式
npm run tauri:dev

# 构建生产版本
npm run tauri:build
```

---

## 📊 Tauri vs Electron MVP 对比

| 项目 | MVP (Electron) | Tauri (目标) |
|------|----------------|--------------|
| 位置 | clipmaster-mvp | clipmaster-tauri |
| 框架 | Electron 42 | Tauri 2.0 |
| 前端 | Svelte 5 | Svelte 5 |
| 数据库 | sql.js | rusqlite |
| 内存 | ~120MB | ~40MB (-66%) |
| 体积 | 未打包 | 5-10MB |

---

## ⚠️ 常见问题

### 问题1：tauri init 需要交互式终端
**解决：** 手动创建配置文件

### 问题2：Rust 环境变量未生效
**解决：** 刷新 PowerShell 或重新打开终端

### 问题3：编译错误
**解决：** 确保 Visual Studio Build Tools 已安装

---

## 📝 下一步

1. ⏳ 完成 Tauri 初始化
2. 创建基础文件结构
3. 从 MVP 迁移剪贴板监听功能
4. 从 MVP 迁移数据库功能
5. 从 MVP 迁移 UI 界面
6. 测试和优化

---

**创建时间：** 2024-06-05 10:20  
**状态：** 初始化中  
**进度：** 60%
