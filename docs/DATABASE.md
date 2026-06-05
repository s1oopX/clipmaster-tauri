# Database

ClipMaster 使用 SQLite，数据库文件位于 Tauri 应用数据目录：

```text
%APPDATA%/com.clipmaster.app/clipboard.db
```

当前没有迁移版本表。后续改表结构前，应先补迁移机制。

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
  preview TEXT,
  timestamp INTEGER NOT NULL,
  source_app TEXT,
  is_favorite INTEGER DEFAULT 0,
  is_pinned INTEGER DEFAULT 0,
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
```

## 图片存储

图片保存为 PNG 文件，只在数据库中保存相对路径。

```text
%APPDATA%/com.clipmaster.app/
  clipboard.db
  images/
    2026-06/
      <hash8>_<timestamp>.png
```

示例：

```text
images/2026-06/4f8a91c0_1780650000.png
```

前端通过 `get_app_data_dir` 获取数据目录，再用 Tauri `convertFileSrc` 转为可显示 URL。

## 去重策略

后端保存前会检查 `content_hash`，当前时间窗口为 5 分钟：

```sql
SELECT COUNT(*)
FROM clipboard_items
WHERE content_hash = ?1 AND timestamp > ?2;
```

文本 hash 使用完整文本；图片 hash 使用宽高和采样字节。

## 当前限制

- 没有迁移表。
- 删除图片记录时还没有同步删除图片文件。
- 搜索使用 `LIKE`，大量文本时需要升级 FTS5。
- 没有自动清理策略。

## 后续建议

- 增加 `schema_migrations` 表。
- 增加清理任务：按最大条数和最大保留天数清理普通记录。
- 增加孤儿图片扫描和删除。
- 评估图片缩略图和 WebP 压缩。
