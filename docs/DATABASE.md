# Database

ClipMaster 使用 SQLite，数据库文件位于 Tauri 应用数据目录：

```text
%APPDATA%/com.clipmaster.desktop/clipboard.db
```

旧版数据目录 `%APPDATA%/com.clipmaster.app/` 会在启动时尽量迁移到新目录；如果新目录已有数据，则只移动没有冲突的旧文件，不覆盖新数据。

数据库使用 `schema_migrations` 记录已执行的 schema 和数据迁移。后续改表结构时，需要追加新版本并补测试。

## `sessions`

记录每次应用启动产生的会话。

```sql
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  start_time INTEGER NOT NULL,
  end_time INTEGER,
  item_count INTEGER DEFAULT 0,
  is_active INTEGER DEFAULT 1
);
```

索引：

```sql
CREATE INDEX IF NOT EXISTS idx_session_time ON sessions(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_session_active ON sessions(is_active);
```

## `clipboard_items`

记录剪贴板历史。

```sql
CREATE TABLE IF NOT EXISTS clipboard_items (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  content TEXT,
  image_path TEXT,
  thumbnail_path TEXT,
  preview TEXT,
  timestamp INTEGER NOT NULL,
  date_key TEXT NOT NULL,
  source_app TEXT,
  is_favorite INTEGER DEFAULT 0,
  is_pinned INTEGER DEFAULT 0,
  annotation TEXT,
  content_hash TEXT NOT NULL,
  session_id TEXT NOT NULL,
  FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

索引：

```sql
CREATE INDEX IF NOT EXISTS idx_timestamp
  ON clipboard_items(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_type
  ON clipboard_items(type);

CREATE INDEX IF NOT EXISTS idx_session
  ON clipboard_items(session_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_pinned_fav
  ON clipboard_items(is_pinned DESC, is_favorite DESC, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_content_hash
  ON clipboard_items(content_hash, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_date_key_time
  ON clipboard_items(date_key, is_pinned DESC, timestamp DESC);
```

## `schema_migrations`

记录已经执行过的迁移版本。

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at INTEGER NOT NULL
);
```

当前迁移：

```text
1 add_thumbnail_path
2 add_date_key
3 add_annotation
4 backfill_date_keys
5 migrate_image_paths_to_daily
6 migrate_text_urls_to_links
```

第 6 版迁移会把旧的单 URL 文本记录转换为 `type = 'link'`，并清理首尾空白、重建预览和链接专用 hash。

## 记录类型

`clipboard_items.type` 当前支持：

- `text`：普通文本
- `link`：完整的 `http` 或 `https` 链接
- `image`：图片或截图
- `file`：预留文件类型

## 图片存储

图片保存为 PNG 文件，只在数据库中保存相对路径。

```text
%APPDATA%/com.clipmaster.desktop/
  clipboard.db
  settings.json
  images/
    2026-06-07/
      <hash8>_<timestamp>.png
      <hash8>_<timestamp>_thumb.png
```

示例：

```text
images/2026-06-07/4f8a91c0_1780650000.png
```

前端通过 `get_app_data_dir` 获取数据目录，再用 Tauri `convertFileSrc` 转为可显示 URL。

删除单条图片记录、清空会话和自定义清理会 best-effort 删除对应原图和缩略图。置顶和收藏记录不会被自定义清理选为候选。

## 去重策略

后端保存前会检查 `content_hash`，当前时间窗口为 5 分钟：

```sql
SELECT COUNT(*)
FROM clipboard_items
WHERE content_hash = ?1 AND timestamp > ?2;
```

普通文本 hash 使用完整文本；链接 hash 使用 `link:` 前缀加规范化 URL，避免和普通文本 hash 混淆。图片 hash 使用宽高和采样字节。

## 全文搜索

第 7 版迁移建立 trigram 分词的 FTS5 外容表加速子串搜索（含中文）：

```sql
CREATE VIRTUAL TABLE clipboard_items_fts USING fts5(
  content, preview, annotation,
  content='clipboard_items',
  content_rowid='rowid',
  tokenize='trigram'
);
```

- `AFTER INSERT` / `AFTER DELETE` / `AFTER UPDATE OF content, preview, annotation` 三个触发器保持索引与主表同步，收藏/置顶等状态更新不会触碰索引。
- 查询 ≥ 3 个字符时走 `MATCH "<子串>"`（trigram 短语即子串匹配，大小写不敏感）；不足 3 个字符回退 `LIKE`，语义一致。
- 外容表按 rowid 关联主表。当前代码没有 `VACUUM`；若未来引入，需要在其后执行 `INSERT INTO clipboard_items_fts(clipboard_items_fts) VALUES('rebuild')`。

## 当前限制

- 没有周期后台清理任务；当前清理由设置保存或手动按钮触发。
- 没有孤儿图片扫描。

## 后续建议

- 后续 schema 变更继续追加 `schema_migrations` 版本和旧库升级测试。
- 增加周期清理任务：按最大条数和最大保留天数清理普通记录。
- 增加孤儿图片扫描和删除。
- 评估图片缩略图和 WebP 压缩。
