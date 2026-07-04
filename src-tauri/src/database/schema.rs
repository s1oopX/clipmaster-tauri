use anyhow::Result;
use rusqlite::Connection;

/// 创建表结构
pub(super) fn create_tables(conn: &Connection) -> Result<()> {
    // 创建会话表
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

    // 创建剪贴板记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_items (
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
            )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            )",
        [],
    )?;

    Ok(())
}

pub(super) fn create_indexes(conn: &Connection) -> Result<()> {
    // 创建索引（使用 execute_batch 避免返回结果）
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_timestamp ON clipboard_items(timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_type ON clipboard_items(type);
             CREATE INDEX IF NOT EXISTS idx_session ON clipboard_items(session_id, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_pinned_fav ON clipboard_items(is_pinned DESC, is_favorite DESC, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_content_hash ON clipboard_items(content_hash, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_date_key_time ON clipboard_items(date_key, is_pinned DESC, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_session_time ON sessions(start_time DESC);
             CREATE INDEX IF NOT EXISTS idx_session_active ON sessions(is_active);"
    )?;

    Ok(())
}
