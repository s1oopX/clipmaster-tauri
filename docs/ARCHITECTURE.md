# ClipMaster 技术架构文档

> Tauri + Rust + Svelte 架构设计

## 🏗️ 整体架构

### 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      前端层 (Svelte)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  UI 组件层   │  │  状态管理    │  │  API 封装    │      │
│  │  .svelte     │  │  stores      │  │  api.js      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ IPC 通信 (Tauri Commands)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    后端层 (Rust/Tauri)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  命令层      │  │  业务逻辑层  │  │  数据访问层  │      │
│  │  commands.rs │  │  services/   │  │  database.rs │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      数据存储层                              │
│  ┌──────────────┐            ┌──────────────┐              │
│  │  SQLite      │            │  文件系统    │              │
│  │  clipboard.db│            │  images/     │              │
│  └──────────────┘            └──────────────┘              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      系统层                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  剪贴板 API  │  │  文件系统    │  │  系统托盘    │      │
│  │  clipboard-rs│  │  std::fs     │  │  tray-icon   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

---

## 📦 模块设计

### 前端模块 (src/)

#### 1. UI 组件层

```
src/lib/components/
├── ClipboardList.svelte       # 剪贴板列表（虚拟滚动）
├── ClipboardItem.svelte       # 单条记录组件
├── SessionSidebar.svelte      # 会话侧边栏
├── SessionInfoCard.svelte     # 会话信息卡片
├── SearchBar.svelte           # 搜索框组件
├── FilterPanel.svelte         # 分类筛选面板
├── ImageItem.svelte           # 图片记录组件
├── SettingsDialog.svelte      # 设置对话框
└── PinnedWindow.svelte        # 置顶窗口（计划中）
```

**职责**:
- 纯 UI 展示
- 用户交互事件处理
- 接收 props 和触发事件

---

#### 2. 状态管理层

```javascript
// src/lib/stores/clipboardStore.js
import { writable, derived } from 'svelte/store';

export const clipboardStore = writable({
  items: [],              // 剪贴板记录列表
  loading: false,         // 加载状态
  searchQuery: '',        // 搜索关键词
  filter: 'all',          // 当前筛选（all/text/image/link/code）
});

export const sessionStore = writable({
  currentSession: null,   // 当前会话
  sessions: [],           // 历史会话列表
  sessionFilter: 'current' // 会话筛选（current/previous/today/all）
});

// 派生 store：过滤后的列表
export const filteredItems = derived(
  [clipboardStore, sessionStore],
  ([$clipboard, $session]) => {
    // 根据会话和分类筛选逻辑
  }
);
```

**职责**:
- 集中管理应用状态
- 提供响应式数据
- 派生计算属性

---

#### 3. API 封装层

```javascript
// src/lib/api.js
import { invoke } from '@tauri-apps/api/core';

export const clipboardApi = {
  // 获取剪贴板列表
  async getItems(limit = 100, offset = 0) {
    return await invoke('get_clipboard_items', { limit, offset });
  },
  
  // 搜索
  async searchItems(query, sessionId = null) {
    return await invoke('search_items', { query, sessionId });
  },
  
  // 删除记录
  async deleteItem(itemId) {
    return await invoke('delete_item', { itemId });
  }
};

export const sessionApi = {
  // 获取当前会话
  async getCurrentSession() {
    return await invoke('get_current_session');
  },
  
  // 获取所有会话
  async getSessions() {
    return await invoke('get_sessions');
  },
  
  // 清空会话
  async clearSession(sessionId) {
    return await invoke('clear_session', { sessionId });
  }
};
```

**职责**:
- 封装所有后端调用
- 统一错误处理
- 提供类型提示

---

### 后端模块 (src-tauri/src/)

#### 1. 主程序入口

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod clipboard;
mod database;
mod session;
mod models;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 初始化数据库
            let db = database::Database::new()?;
            app.manage(db);
            
            // 启动剪贴板监听服务
            let clipboard_service = clipboard::ClipboardService::new();
            clipboard_service.start(app.handle());
            
            // 创建会话
            let session_manager = session::SessionManager::new();
            session_manager.start_new_session()?;
            app.manage(session_manager);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_clipboard_items,
            commands::get_current_session,
            commands::search_items,
            // ... 其他命令
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

#### 2. 剪贴板监听服务

```rust
// src-tauri/src/clipboard/mod.rs
use clipboard_rs::{Clipboard, ClipboardContext};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

pub struct ClipboardService {
    ctx: Arc<Mutex<ClipboardContext>>,
    last_hash: Arc<Mutex<String>>,
}

impl ClipboardService {
    pub fn new() -> Self {
        Self {
            ctx: Arc::new(Mutex::new(ClipboardContext::new().unwrap())),
            last_hash: Arc::new(Mutex::new(String::new())),
        }
    }
    
    pub fn start(&self, app_handle: tauri::AppHandle) {
        let ctx = Arc::clone(&self.ctx);
        let last_hash = Arc::clone(&self.last_hash);
        
        tokio::spawn(async move {
            loop {
                // 读取剪贴板内容
                if let Ok(content) = Self::get_clipboard_content(&ctx) {
                    let hash = Self::calculate_hash(&content);
                    
                    // 检查是否重复
                    let mut last = last_hash.lock().unwrap();
                    if hash != *last {
                        *last = hash;
                        
                        // 保存到数据库
                        Self::save_clipboard_item(&app_handle, content).await;
                    }
                }
                
                sleep(Duration::from_millis(500)).await;
            }
        });
    }
    
    fn get_clipboard_content(ctx: &Arc<Mutex<ClipboardContext>>) -> Result<ClipboardContent, Error> {
        // 检测剪贴板内容类型
        // 返回文本、图片或文件
    }
    
    fn calculate_hash(content: &ClipboardContent) -> String {
        // MD5 哈希计算
    }
    
    async fn save_clipboard_item(app_handle: &tauri::AppHandle, content: ClipboardContent) {
        // 获取数据库实例
        // 插入记录
        // 触发前端更新事件
    }
}
```

**职责**:
- 轮询检测剪贴板变化
- 内容去重（MD5 + 时间窗口）
- 异步保存到数据库
- 通知前端更新

---

#### 3. 会话管理器

```rust
// src-tauri/src/session/mod.rs
use rusqlite::Connection;
use chrono::Utc;
use nanoid::nanoid;

pub struct SessionManager {
    current_session_id: Mutex<Option<String>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            current_session_id: Mutex::new(None),
        }
    }
    
    /// 启动新会话
    pub fn start_new_session(&self, db: &Connection) -> Result<String, Error> {
        let session_id = nanoid!();
        let now = Utc::now().timestamp_millis();
        
        // 结束旧的活跃会话
        db.execute(
            "UPDATE sessions SET is_active = 0, end_time = ? WHERE is_active = 1",
            [now],
        )?;
        
        // 创建新会话
        db.execute(
            "INSERT INTO sessions (id, start_time, is_active) VALUES (?, ?, 1)",
            [&session_id, &now.to_string()],
        )?;
        
        *self.current_session_id.lock().unwrap() = Some(session_id.clone());
        
        Ok(session_id)
    }
    
    /// 结束会话
    pub fn end_session(&self, db: &Connection, session_id: &str) -> Result<(), Error> {
        let now = Utc::now().timestamp_millis();
        
        // 统计记录数
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE session_id = ?",
            [session_id],
            |row| row.get(0),
        )?;
        
        // 更新会话
        db.execute(
            "UPDATE sessions SET end_time = ?, item_count = ?, is_active = 0 WHERE id = ?",
            [&now.to_string(), &count.to_string(), session_id],
        )?;
        
        Ok(())
    }
    
    /// 获取当前会话ID
    pub fn get_current_session_id(&self) -> Option<String> {
        self.current_session_id.lock().unwrap().clone()
    }
}
```

**职责**:
- 管理会话生命周期
- 启动时创建会话
- 退出时结束会话
- 提供会话查询接口

---

#### 4. 数据库访问层

```rust
// src-tauri/src/database/mod.rs
use rusqlite::{Connection, params};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 初始化数据库
    pub fn new() -> Result<Self, Error> {
        let data_dir = Self::get_data_dir()?;
        let db_path = data_dir.join("clipboard.db");
        
        let conn = Connection::open(db_path)?;
        
        // 创建表
        Self::create_tables(&conn)?;
        
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
    
    /// 创建表结构
    fn create_tables(conn: &Connection) -> Result<(), Error> {
        // 会话表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                item_count INTEGER DEFAULT 0,
                is_active INTEGER DEFAULT 1
            )",
            [],
        )?;
        
        // 剪贴板表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                content TEXT,
                image_path TEXT,
                preview TEXT,
                timestamp INTEGER NOT NULL,
                source_app TEXT,
                is_favorite INTEGER DEFAULT 0,
                is_pinned INTEGER DEFAULT 0,
                content_hash TEXT,
                session_id TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            )",
            [],
        )?;
        
        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session ON clipboard_items(session_id, timestamp DESC)",
            [],
        )?;
        
        Ok(())
    }
    
    /// 插入剪贴板记录
    pub fn insert_item(&self, item: &ClipboardItem) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clipboard_items (id, type, content, timestamp, session_id, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                item.id,
                item.type_,
                item.content,
                item.timestamp,
                item.session_id,
                item.content_hash
            ],
        )?;
        Ok(())
    }
    
    /// 按会话查询记录
    pub fn get_items_by_session(
        &self,
        session_id: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ClipboardItem>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM clipboard_items 
             WHERE session_id = ?1 
             ORDER BY is_pinned DESC, timestamp DESC 
             LIMIT ?2 OFFSET ?3"
        )?;
        
        let items = stmt.query_map(params![session_id, limit, offset], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                type_: row.get(1)?,
                content: row.get(2)?,
                // ... 其他字段
            })
        })?;
        
        Ok(items.collect::<Result<Vec<_>, _>>()?)
    }
}
```

**职责**:
- 数据库连接管理
- 表结构创建和迁移
- CRUD 操作封装
- 查询优化

---

#### 5. Tauri Commands

```rust
// src-tauri/src/commands.rs
use tauri::State;
use crate::database::Database;
use crate::session::SessionManager;
use crate::models::*;

#[tauri::command]
pub async fn get_clipboard_items(
    db: State<'_, Database>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items(limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_session(
    session_mgr: State<'_, SessionManager>,
    db: State<'_, Database>,
) -> Result<Option<Session>, String> {
    if let Some(session_id) = session_mgr.get_current_session_id() {
        db.get_session(&session_id)
            .map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn search_items(
    db: State<'_, Database>,
    query: String,
    session_id: Option<String>,
) -> Result<Vec<ClipboardItem>, String> {
    db.search_items(&query, session_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_item(
    db: State<'_, Database>,
    item_id: String,
) -> Result<(), String> {
    db.delete_item(&item_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_session(
    db: State<'_, Database>,
    session_id: String,
) -> Result<(), String> {
    db.clear_session(&session_id)
        .map_err(|e| e.to_string())
}
```

**职责**:
- 暴露前端可调用的命令
- 参数验证和转换
- 错误处理和返回

---

## 🔄 数据流

### 剪贴板监听流程

```
1. 用户复制内容
   ↓
2. ClipboardService 检测到变化（500ms 轮询）
   ↓
3. 计算 MD5 哈希，检查是否重复
   ↓
4. 获取当前 session_id
   ↓
5. 保存到 SQLite 数据库
   ↓
6. 触发前端事件 (emit('clipboard:new-item'))
   ↓
7. 前端更新 clipboardStore
   ↓
8. UI 自动更新（Svelte 响应式）
```

### 会话查询流程

```
1. 用户点击"本次会话"
   ↓
2. 调用 sessionApi.getCurrentSession()
   ↓
3. Tauri invoke('get_current_session')
   ↓
4. Rust: SessionManager.get_current_session_id()
   ↓
5. Rust: Database.get_items_by_session()
   ↓
6. 返回记录列表到前端
   ↓
7. 更新 clipboardStore.items
   ↓
8. ClipboardList 组件渲染
```

---

## 🛠️ 技术选型理由

### Tauri vs Electron

| 维度 | Tauri | Electron | 原因 |
|------|-------|----------|------|
| 内存占用 | ~40MB | ~120MB | Tauri 使用系统 WebView |
| 打包体积 | 5-10MB | 80-120MB | 不包含 Chromium |
| 启动速度 | < 1s | 1-2s | Rust 编译后性能更好 |
| 安全性 | 更高 | 较低 | Rust 内存安全 + IPC 隔离 |

### Svelte vs React/Vue

- **更小的打包体积** - 编译时框架，无运行时
- **更快的渲染速度** - 无虚拟 DOM
- **更简洁的语法** - 更少的样板代码
- **响应式简单** - 原生响应式系统

### rusqlite vs sled/redb

- **成熟稳定** - SQLite 生态成熟
- **标准 SQL** - 熟悉的查询语法
- **FTS5 全文搜索** - 内置全文搜索引擎
- **跨平台兼容** - 广泛支持

---

## 📊 性能指标

### 内存占用

- **空闲**: < 40MB
- **活跃**: < 60MB
- **5000 条记录**: < 100MB

### 启动时间

- **冷启动**: < 1 秒
- **热启动**: < 0.3 秒

### 响应速度

- **剪贴板检测延迟**: < 500ms
- **搜索响应**: < 100ms
- **列表滚动**: 60 FPS

---

**文档版本**: v1.0  
**创建日期**: 2026-06-05  
**最后更新**: 2026-06-05
