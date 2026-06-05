# ClipMaster API 接口文档

> Tauri Commands - 前后端通信接口

## 📡 接口概览

ClipMaster 使用 Tauri 的 IPC 通信机制，通过 `invoke` 调用后端 Rust 函数。

### 接口分类

- **剪贴板管理** - 增删查改剪贴板记录
- **会话管理** - 查询和管理会话
- **搜索功能** - 全文搜索
- **设置管理** - 应用配置

---

## 🔹 剪贴板管理

### 1. get_clipboard_items

**描述**: 获取剪贴板记录列表

**命令名**: `get_clipboard_items`

#### 请求参数

```typescript
interface GetClipboardItemsParams {
  limit?: number;    // 返回记录数，默认 100
  offset?: number;   // 偏移量，默认 0（用于分页）
}
```

#### 返回值

```typescript
interface ClipboardItem {
  id: string;              // 记录ID
  type: 'text' | 'image' | 'file';
  content: string | null;  // 文本内容
  image_path: string | null;
  preview: string | null;  // 预览文本
  timestamp: number;       // Unix 毫秒时间戳
  source_app: string | null;
  is_favorite: boolean;
  is_pinned: boolean;
  content_hash: string;
  session_id: string;
}

type Response = ClipboardItem[];
```

#### 前端调用示例

```javascript
import { invoke } from '@tauri-apps/api/core';

// 获取最新 100 条记录
const items = await invoke('get_clipboard_items', {
  limit: 100,
  offset: 0
});

// 分页：获取第二页（101-200）
const page2 = await invoke('get_clipboard_items', {
  limit: 100,
  offset: 100
});
```

#### Rust 实现

```rust
#[tauri::command]
pub async fn get_clipboard_items(
    db: State<'_, Database>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items(limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}
```

---

### 2. delete_item

**描述**: 删除单条剪贴板记录

**命令名**: `delete_item`

#### 请求参数

```typescript
interface DeleteItemParams {
  item_id: string;  // 记录ID
}
```

#### 返回值

```typescript
type Response = void;
```

#### 前端调用示例

```javascript
await invoke('delete_item', { item_id: 'item_abc123' });
```

#### Rust 实现

```rust
#[tauri::command]
pub async fn delete_item(
    db: State<'_, Database>,
    item_id: String,
) -> Result<(), String> {
    db.delete_item(&item_id)
        .map_err(|e| e.to_string())
}
```

---

### 3. toggle_favorite

**描述**: 切换收藏状态

**命令名**: `toggle_favorite`

#### 请求参数

```typescript
interface ToggleFavoriteParams {
  item_id: string;
}
```

#### 返回值

```typescript
type Response = boolean;  // 新的收藏状态
```

#### 前端调用示例

```javascript
const isFavorite = await invoke('toggle_favorite', { 
  item_id: 'item_abc123' 
});
```

#### Rust 实现

```rust
#[tauri::command]
pub async fn toggle_favorite(
    db: State<'_, Database>,
    item_id: String,
) -> Result<bool, String> {
    db.toggle_favorite(&item_id)
        .map_err(|e| e.to_string())
}
```

---

### 4. toggle_pinned

**描述**: 切换置顶状态

**命令名**: `toggle_pinned`

#### 请求参数

```typescript
interface TogglePinnedParams {
  item_id: string;
}
```

#### 返回值

```typescript
type Response = boolean;  // 新的置顶状态
```

#### 前端调用示例

```javascript
const isPinned = await invoke('toggle_pinned', { 
  item_id: 'item_abc123' 
});
```

---

## 🔹 会话管理

### 5. get_current_session

**描述**: 获取当前活跃会话

**命令名**: `get_current_session`

#### 请求参数

无

#### 返回值

```typescript
interface Session {
  id: string;
  start_time: number;    // Unix 毫秒时间戳
  end_time: number | null;
  item_count: number;
  is_active: boolean;
}

type Response = Session | null;
```

#### 前端调用示例

```javascript
const currentSession = await invoke('get_current_session');

if (currentSession) {
  console.log(`会话ID: ${currentSession.id}`);
  console.log(`记录数: ${currentSession.item_count}`);
}
```

#### Rust 实现

```rust
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
```

---

### 6. get_sessions

**描述**: 获取历史会话列表

**命令名**: `get_sessions`

#### 请求参数

```typescript
interface GetSessionsParams {
  limit?: number;  // 返回数量，默认 50
}
```

#### 返回值

```typescript
type Response = Session[];
```

#### 前端调用示例

```javascript
// 获取最近 20 个会话
const sessions = await invoke('get_sessions', { limit: 20 });
```

---

### 7. get_items_by_session

**描述**: 获取指定会话的记录

**命令名**: `get_items_by_session`

#### 请求参数

```typescript
interface GetItemsBySessionParams {
  session_id: string;
  limit?: number;
  offset?: number;
}
```

#### 返回值

```typescript
type Response = ClipboardItem[];
```

#### 前端调用示例

```javascript
const items = await invoke('get_items_by_session', {
  session_id: 'session_20260605_143015',
  limit: 100,
  offset: 0
});
```

#### Rust 实现

```rust
#[tauri::command]
pub async fn get_items_by_session(
    db: State<'_, Database>,
    session_id: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items_by_session(
        &session_id,
        limit.unwrap_or(100),
        offset.unwrap_or(0),
    )
    .map_err(|e| e.to_string())
}
```

---

### 8. get_items_by_time_range

**描述**: 获取指定时间范围内的记录

**命令名**: `get_items_by_time_range`

#### 请求参数

```typescript
interface GetItemsByTimeRangeParams {
  start_time: number;  // Unix 毫秒时间戳
  end_time: number;
  limit?: number;
}
```

#### 返回值

```typescript
type Response = ClipboardItem[];
```

#### 前端调用示例

```javascript
// 获取今天的所有记录
const todayStart = new Date().setHours(0, 0, 0, 0);
const todayEnd = new Date().setHours(23, 59, 59, 999);

const items = await invoke('get_items_by_time_range', {
  start_time: todayStart,
  end_time: todayEnd,
  limit: 500
});
```

---

### 9. clear_session

**描述**: 清空指定会话的所有记录

**命令名**: `clear_session`

#### 请求参数

```typescript
interface ClearSessionParams {
  session_id: string;
}
```

#### 返回值

```typescript
type Response = void;
```

#### 前端调用示例

```javascript
await invoke('clear_session', {
  session_id: 'session_20260605_143015'
});
```

#### Rust 实现

```rust
#[tauri::command]
pub async fn clear_session(
    db: State<'_, Database>,
    session_id: String,
) -> Result<(), String> {
    db.clear_session(&session_id)
        .map_err(|e| e.to_string())
}
```

---

## 🔹 搜索功能

### 10. search_items

**描述**: 全文搜索剪贴板记录

**命令名**: `search_items`

#### 请求参数

```typescript
interface SearchItemsParams {
  query: string;         // 搜索关键词
  session_id?: string;   // 可选：限定在某个会话内搜索
  limit?: number;
}
```

#### 返回值

```typescript
type Response = ClipboardItem[];
```

#### 前端调用示例

```javascript
// 全局搜索
const results = await invoke('search_items', {
  query: 'console.log',
  limit: 100
});

// 会话内搜索
const sessionResults = await invoke('search_items', {
  query: 'console.log',
  session_id: 'session_20260605_143015',
  limit: 50
});
```

#### Rust 实现

```rust
#[tauri::command]
pub async fn search_items(
    db: State<'_, Database>,
    query: String,
    session_id: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.search_items(&query, session_id.as_deref(), limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}
```

---

## 🔹 设置管理

### 11. get_settings

**描述**: 获取应用设置

**命令名**: `get_settings`

#### 请求参数

无

#### 返回值

```typescript
interface Settings {
  max_items: number;           // 最大记录数
  auto_cleanup: boolean;       // 自动清理
  cleanup_days: number;        // 保留天数
  start_on_boot: boolean;      // 开机自启
  hotkey_show: string;         // 显示快捷键
  hotkey_screenshot: string;   // 截图快捷键
}

type Response = Settings;
```

#### 前端调用示例

```javascript
const settings = await invoke('get_settings');
```

---

### 12. update_settings

**描述**: 更新应用设置

**命令名**: `update_settings`

#### 请求参数

```typescript
interface UpdateSettingsParams {
  settings: Partial<Settings>;  // 部分更新
}
```

#### 返回值

```typescript
type Response = void;
```

#### 前端调用示例

```javascript
await invoke('update_settings', {
  settings: {
    max_items: 5000,
    auto_cleanup: true
  }
});
```

---

## 🔹 事件监听

### clipboard:new-item

**描述**: 新剪贴板记录事件（后端 → 前端）

#### 事件数据

```typescript
interface ClipboardNewItemEvent {
  item: ClipboardItem;
}
```

#### 前端监听示例

```javascript
import { listen } from '@tauri-apps/api/event';

// 监听新记录事件
const unlisten = await listen('clipboard:new-item', (event) => {
  const item = event.payload.item;
  console.log('新的剪贴板记录:', item);
  
  // 更新 UI
  clipboardStore.addItem(item);
});

// 取消监听
unlisten();
```

#### Rust 触发示例

```rust
use tauri::Manager;

// 触发前端事件
app.emit_all("clipboard:new-item", ClipboardNewItemPayload {
    item: new_item,
})?;
```

---

## 📦 前端 API 封装

### api.js

```javascript
// src/lib/api.js
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * 剪贴板 API
 */
export const clipboardApi = {
  /**
   * 获取剪贴板列表
   * @param {number} limit - 返回数量
   * @param {number} offset - 偏移量
   */
  async getItems(limit = 100, offset = 0) {
    return await invoke('get_clipboard_items', { limit, offset });
  },

  /**
   * 删除记录
   * @param {string} itemId
   */
  async deleteItem(itemId) {
    return await invoke('delete_item', { item_id: itemId });
  },

  /**
   * 切换收藏
   * @param {string} itemId
   * @returns {boolean} 新的收藏状态
   */
  async toggleFavorite(itemId) {
    return await invoke('toggle_favorite', { item_id: itemId });
  },

  /**
   * 切换置顶
   * @param {string} itemId
   * @returns {boolean} 新的置顶状态
   */
  async togglePinned(itemId) {
    return await invoke('toggle_pinned', { item_id: itemId });
  },

  /**
   * 监听新记录事件
   * @param {Function} callback
   * @returns {Promise<Function>} unlisten 函数
   */
  async onNewItem(callback) {
    return await listen('clipboard:new-item', (event) => {
      callback(event.payload.item);
    });
  },
};

/**
 * 会话 API
 */
export const sessionApi = {
  /**
   * 获取当前会话
   */
  async getCurrentSession() {
    return await invoke('get_current_session');
  },

  /**
   * 获取历史会话列表
   * @param {number} limit
   */
  async getSessions(limit = 50) {
    return await invoke('get_sessions', { limit });
  },

  /**
   * 按会话获取记录
   * @param {string} sessionId
   * @param {number} limit
   * @param {number} offset
   */
  async getItemsBySession(sessionId, limit = 100, offset = 0) {
    return await invoke('get_items_by_session', {
      session_id: sessionId,
      limit,
      offset,
    });
  },

  /**
   * 按时间范围获取记录
   * @param {number} startTime - Unix 时间戳（毫秒）
   * @param {number} endTime
   * @param {number} limit
   */
  async getItemsByTimeRange(startTime, endTime, limit = 500) {
    return await invoke('get_items_by_time_range', {
      start_time: startTime,
      end_time: endTime,
      limit,
    });
  },

  /**
   * 清空会话
   * @param {string} sessionId
   */
  async clearSession(sessionId) {
    return await invoke('clear_session', { session_id: sessionId });
  },
};

/**
 * 搜索 API
 */
export const searchApi = {
  /**
   * 搜索记录
   * @param {string} query - 搜索关键词
   * @param {string} sessionId - 可选：会话ID
   * @param {number} limit
   */
  async searchItems(query, sessionId = null, limit = 100) {
    return await invoke('search_items', {
      query,
      session_id: sessionId,
      limit,
    });
  },
};

/**
 * 设置 API
 */
export const settingsApi = {
  /**
   * 获取设置
   */
  async getSettings() {
    return await invoke('get_settings');
  },

  /**
   * 更新设置
   * @param {Object} settings
   */
  async updateSettings(settings) {
    return await invoke('update_settings', { settings });
  },
};
```

---

## 🔧 错误处理

### 错误格式

所有 API 调用失败时返回字符串错误信息：

```typescript
try {
  const items = await invoke('get_clipboard_items');
} catch (error) {
  console.error('API 调用失败:', error);
  // error 是字符串类型
}
```

### 常见错误码

| 错误信息 | 原因 | 解决方案 |
|---------|------|----------|
| "Database error: ..." | 数据库操作失败 | 检查数据库文件权限 |
| "Session not found" | 会话不存在 | 检查 session_id 是否正确 |
| "Item not found" | 记录不存在 | 检查 item_id 是否正确 |
| "Invalid parameter: ..." | 参数错误 | 检查参数类型和范围 |

---

## 📊 性能建议

### 1. 分页加载

```javascript
// ✅ 推荐：分页加载
const items = await clipboardApi.getItems(100, 0);

// ❌ 不推荐：一次性加载大量数据
const items = await clipboardApi.getItems(10000, 0);
```

### 2. 批量操作

```javascript
// ❌ 不推荐：循环单个删除
for (const id of itemIds) {
  await clipboardApi.deleteItem(id);
}

// ✅ 推荐：后续实现批量删除接口
// await clipboardApi.deleteBatch(itemIds);
```

### 3. 防抖搜索

```javascript
import { debounce } from 'lodash-es';

const debouncedSearch = debounce(async (query) => {
  const results = await searchApi.searchItems(query);
  updateUI(results);
}, 300);

searchInput.addEventListener('input', (e) => {
  debouncedSearch(e.target.value);
});
```

---

**文档版本**: v1.0  
**创建日期**: 2026-06-05  
**最后更新**: 2026-06-05
