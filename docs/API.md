# API

ClipMaster 前端通过 Tauri `invoke` 调用 Rust command，通过 `listen` 接收后端事件。本文只记录当前代码中已经注册的接口。

## 数据类型

```typescript
type ClipboardType = 'text' | 'image' | 'file';

interface ClipboardItem {
  id: string;
  type: ClipboardType;
  content: string | null;
  image_path: string | null;
  preview: string | null;
  timestamp: number;
  source_app: string | null;
  is_favorite: boolean;
  is_pinned: boolean;
  content_hash: string;
  session_id: string;
}

interface Session {
  id: string;
  start_time: number;
  end_time: number | null;
  item_count: number;
  is_active: boolean;
}
```

## 剪贴板接口

### `get_clipboard_items`

获取剪贴板记录。

```typescript
invoke('get_clipboard_items', {
  limit?: number,
  offset?: number
}) => Promise<ClipboardItem[]>
```

### `delete_item`

删除单条记录。

```typescript
invoke('delete_item', {
  itemId: string
}) => Promise<void>
```

### `toggle_favorite`

切换收藏状态，返回新状态。

```typescript
invoke('toggle_favorite', {
  itemId: string
}) => Promise<boolean>
```

### `toggle_pinned`

切换置顶状态，返回新状态。

```typescript
invoke('toggle_pinned', {
  itemId: string
}) => Promise<boolean>
```

### `copy_to_clipboard`

把文本写回系统剪贴板。

```typescript
invoke('copy_to_clipboard', {
  text: string
}) => Promise<void>
```

当前只支持文本。图片复制是后续任务。

## 会话接口

### `get_current_session`

获取当前活跃会话。

```typescript
invoke('get_current_session') => Promise<Session | null>
```

### `get_sessions`

获取历史会话。

```typescript
invoke('get_sessions', {
  limit?: number
}) => Promise<Session[]>
```

### `get_items_by_session`

获取指定会话的剪贴板记录。

```typescript
invoke('get_items_by_session', {
  sessionId: string,
  limit?: number,
  offset?: number
}) => Promise<ClipboardItem[]>
```

### `clear_session`

删除指定会话和它的记录。

```typescript
invoke('clear_session', {
  sessionId: string
}) => Promise<void>
```

## 搜索接口

### `search_items`

搜索记录。当前实现使用 SQLite `LIKE`，不是 FTS5。

```typescript
invoke('search_items', {
  query: string,
  sessionId?: string | null,
  limit?: number
}) => Promise<ClipboardItem[]>
```

## 文件路径接口

### `get_app_data_dir`

返回 Tauri 应用数据目录，用于前端拼接图片路径后交给 `convertFileSrc`。

```typescript
invoke('get_app_data_dir') => Promise<string>
```

## 事件

### `clipboard:new-item`

后端监听到新剪贴板记录后触发。

```typescript
listen<ClipboardItem>('clipboard:new-item', (event) => {
  const item = event.payload;
});
```

## 前端封装

当前封装在 [src/lib/api.js](../src/lib/api.js)：

- `clipboardApi`
- `sessionApi`
- `searchApi`
- `convertImagePath`

新增 command 时，需要同步更新 Rust `generate_handler!`、前端封装和本文档。
