# API

ClipMaster 前端通过 Tauri `invoke` 调用 Rust command，通过 `listen` 接收后端事件。本文只记录当前代码中已经注册的接口。

## 数据类型

```typescript
type ClipboardType = 'text' | 'link' | 'image' | 'file';
type ClipboardFilterType = 'text' | 'link' | 'image' | 'file';

interface ClipboardItem {
  id: string;
  type: ClipboardType;
  content: string | null;
  image_path: string | null;
  thumbnail_path: string | null;
  preview: string | null;
  timestamp: number;
  date_key: string;
  source_app: string | null;
  is_favorite: boolean;
  is_pinned: boolean;
  annotation: string | null;
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

interface ClipboardDay {
  date_key: string;
  item_count: number;
  start_time: number;
  end_time: number;
}

interface CleanupPlan {
  item_count: number;
  text_count: number;
  image_count: number;
  oldest_timestamp: number | null;
  newest_timestamp: number | null;
}

interface AppSettings {
  clipboard_monitor_enabled: boolean;
  show_main_window_on_start: boolean;
  auto_start_enabled: boolean;
  max_items: number;
  capture_delay_ms: number;
  screenshot_hotkey: string;
  main_window_hotkey: string;
  time_zone: string;
  language: string;
  auto_cleanup_enabled: boolean;
  cleanup_max_items: number;
  cleanup_keep_days: number;
  dev_server_port: number;
}

interface PortCheckResult {
  port: number;
  available: boolean;
  suggested_port: number | null;
  message: string;
}
```

## 剪贴板接口

### `get_clipboard_items`

获取剪贴板记录。

```typescript
invoke('get_clipboard_items', {
  limit?: number,
  offset?: number,
  itemType?: ClipboardFilterType,
  favoriteOnly?: boolean
}) => Promise<ClipboardItem[]>
```

`limit` 会被后端限制在安全范围内，`offset` 用于列表“加载更多”。`itemType` 和 `favoriteOnly` 在数据库查询阶段过滤，避免只筛当前已加载页。

### `get_items_by_day`

按日期获取剪贴板记录。

```typescript
invoke('get_items_by_day', {
  dateKey: string,
  limit?: number,
  offset?: number,
  itemType?: ClipboardFilterType,
  favoriteOnly?: boolean
}) => Promise<ClipboardItem[]>
```

`dateKey` 使用 `YYYY-MM-DD`。类型和收藏筛选同样在后端执行。

### `get_available_days`

获取有记录的日期列表。

```typescript
invoke('get_available_days', {
  limit?: number
}) => Promise<ClipboardDay[]>
```

### `delete_item`

删除单条记录。

```typescript
invoke('delete_item', {
  itemId: string
}) => Promise<void>
```

图片记录删除后会 best-effort 删除对应原图和缩略图。

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

### `copy_image_to_clipboard`

把应用数据目录下的图片写回系统剪贴板。

```typescript
invoke('copy_image_to_clipboard', {
  imagePath: string
}) => Promise<void>
```

### `update_item_content`

更新文本记录内容。

```typescript
invoke('update_item_content', {
  itemId: string,
  newContent: string
}) => Promise<ClipboardItem>
```

如果更新后的内容是完整 `http` 或 `https` 链接，后端会把记录类型改为 `link`，并使用链接专用 hash。

### `update_item_annotation`

更新记录标注，不改写原始剪贴板内容；非空标注会自动收藏记录。

```typescript
invoke('update_item_annotation', {
  itemId: string,
  annotation: string | null
}) => Promise<string | null>
```

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

清空会话时也会 best-effort 删除会话内图片记录对应的原图和缩略图。

## 搜索接口

### `search_items`

搜索记录。当前实现使用 SQLite `LIKE`，不是 FTS5。

```typescript
invoke('search_items', {
  query: string,
  sessionId?: string | null,
  limit?: number,
  dateKey?: string | null,
  offset?: number,
  itemType?: ClipboardFilterType,
  favoriteOnly?: boolean
}) => Promise<ClipboardItem[]>
```

搜索支持分页式增量加载；`itemType` 和 `favoriteOnly` 会参与 SQL 查询。

## 文件路径接口

### `get_app_data_dir`

返回 Tauri 应用数据目录，用于前端拼接图片路径后交给 `convertFileSrc`。

```typescript
invoke('get_app_data_dir') => Promise<string>
```

## 工具接口

### `start_region_screenshot`

打开区域截图选择窗口。后端会在冻结当前屏幕前隐藏可见的主窗口，并在截图结束时按启动前状态恢复主窗口。

```typescript
invoke('start_region_screenshot') => Promise<void>
```

### `save_screenshot_image`

保存截图窗口基于冻结屏幕合成的最终 PNG，写入图片历史，并复制到系统剪贴板。

```typescript
invoke('save_screenshot_image', {
  imageDataUrl: string,
  snapshotPath?: string
}) => Promise<ClipboardItem>
```

### `cleanup_screenshot_snapshot`

取消截图或关闭截图窗口时清理冻结屏幕临时文件。

```typescript
invoke('cleanup_screenshot_snapshot', {
  snapshotPath: string
}) => Promise<void>
```

### `capture_region_screenshot`

旧版兼容兜底接口：根据截图窗口传入的区域直接捕获图片并写入历史记录。当前截图窗口优先使用 `save_screenshot_image` 保存冻结图合成结果。

```typescript
invoke('capture_region_screenshot', {
  x: number,
  y: number,
  width: number,
  height: number
}) => Promise<ClipboardItem>
```

### `pin_image`

将图片记录以置顶小窗打开。

```typescript
invoke('pin_image', {
  imagePath: string
}) => Promise<void>
```

### `open_external_url`

在系统默认浏览器打开安全的外部链接。当前只允许 `http` 和 `https`，会拒绝非 Web scheme、不完整 host、空白字符和控制字符。

```typescript
invoke('open_external_url', {
  url: string
}) => Promise<void>
```

## 设置接口

### `get_settings`

获取应用设置。

```typescript
invoke('get_settings') => Promise<AppSettings>
```

### `save_settings`

保存应用设置。保存时会校验截图快捷键、主窗口快捷键和开发端口，并在时区变化时重建 `date_key`。

```typescript
invoke('save_settings', {
  settings: AppSettings
}) => Promise<AppSettings>
```

### `check_dev_server_port`

检查开发端口是否可用，并在占用时返回建议端口。

```typescript
invoke('check_dev_server_port', {
  port: number
}) => Promise<PortCheckResult>
```

### `restart_app`

请求重启应用。

```typescript
invoke('restart_app') => Promise<void>
```

### `preview_custom_cleanup`

预览自定义清理候选记录。置顶和收藏记录会保留。

```typescript
invoke('preview_custom_cleanup', {
  maxItems: number,
  keepDays: number
}) => Promise<CleanupPlan>
```

### `run_custom_cleanup`

执行自定义清理，并 best-effort 删除图片文件。

```typescript
invoke('run_custom_cleanup', {
  maxItems: number,
  keepDays: number
}) => Promise<CleanupPlan>
```

### `clear_all_history`

清空全部剪贴板历史，包括收藏、置顶、标注记录和图片文件。当前活动会话会保留为空会话，历史会话会被删除。

```typescript
invoke('clear_all_history') => Promise<CleanupPlan>
```

## 事件

### `hotkey:screenshot`

后端全局截图快捷键触发后发送，前端收到后打开区域截图窗口。

```typescript
listen('hotkey:screenshot', () => {
  // start screenshot
});
```

### `hotkey:focus-search`

后端主窗口快捷键触发并显示主窗口后发送，前端收到后关闭设置弹窗并聚焦搜索框。

```typescript
listen('hotkey:focus-search', () => {
  // focus search input
});
```

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
- `toolApi`
- `settingsApi`
- `convertImagePath`

新增 command 时，需要同步更新 Rust `generate_handler!`、前端封装和本文档。
