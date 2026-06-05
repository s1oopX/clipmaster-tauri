use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::{ClipboardItem, ClipboardType, CreateClipboardItem, Session};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 初始化数据库
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        // 确保数据目录存在
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("clipboard.db");
        let conn = Connection::open(&db_path).context("Failed to open database")?;

        // 优化数据库配置（使用 execute_batch）
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA cache_size=-10000;
             PRAGMA synchronous=NORMAL;
             PRAGMA mmap_size=67108864;",
        )?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        // 创建表结构
        db.create_tables()?;

        Ok(db)
    }

    /// 创建表结构
    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

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
                source_app TEXT,
                is_favorite INTEGER DEFAULT 0,
                is_pinned INTEGER DEFAULT 0,
                content_hash TEXT NOT NULL,
                session_id TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            )",
            [],
        )?;

        // 创建索引（使用 execute_batch 避免返回结果）
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON clipboard_items(timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_type ON clipboard_items(type);
             CREATE INDEX IF NOT EXISTS idx_session ON clipboard_items(session_id, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_pinned_fav ON clipboard_items(is_pinned DESC, is_favorite DESC, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_content_hash ON clipboard_items(content_hash, timestamp DESC);
             CREATE INDEX IF NOT EXISTS idx_session_time ON sessions(start_time DESC);
             CREATE INDEX IF NOT EXISTS idx_session_active ON sessions(is_active);"
        )?;

        Ok(())
    }

    /// 插入剪贴板记录
    pub fn insert_item(&self, item: CreateClipboardItem) -> Result<ClipboardItem> {
        let conn = self.conn.lock().unwrap();

        let id = nanoid::nanoid!();
        let timestamp = Utc::now().timestamp_millis();
        let preview = item.content.as_ref().map(|c| {
            // 安全地截取字符串，避免切断多字节字符
            let char_count = c.chars().count();
            if char_count > 100 {
                let preview_text: String = c.chars().take(100).collect();
                format!("{}...", preview_text)
            } else {
                c.clone()
            }
        });

        conn.execute(
            "INSERT INTO clipboard_items (
                id, type, content, image_path, thumbnail_path, preview, timestamp,
                source_app, content_hash, session_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                item.type_.as_str(),
                item.content,
                item.image_path,
                item.thumbnail_path,
                preview,
                timestamp,
                item.source_app,
                item.content_hash,
                item.session_id,
            ],
        )?;

        Ok(ClipboardItem {
            id: id.clone(),
            type_: item.type_,
            content: item.content,
            image_path: item.image_path,
            thumbnail_path: item.thumbnail_path,
            preview,
            timestamp,
            source_app: item.source_app,
            is_favorite: false,
            is_pinned: false,
            content_hash: item.content_hash,
            session_id: item.session_id,
        })
    }

    /// 获取剪贴板记录列表
    pub fn get_items(&self, limit: i32, offset: i32) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, type, content, image_path, thumbnail_path, preview, timestamp,
                    source_app, is_favorite, is_pinned, content_hash, session_id
             FROM clipboard_items
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let items = stmt
            .query_map(params![limit, offset], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    type_: ClipboardType::from_str(&row.get::<_, String>(1)?)
                        .unwrap_or(ClipboardType::Text),
                    content: row.get(2)?,
                    image_path: row.get(3)?,
                    thumbnail_path: row.get(4)?,
                    preview: row.get(5)?,
                    timestamp: row.get(6)?,
                    source_app: row.get(7)?,
                    is_favorite: row.get::<_, i32>(8)? == 1,
                    is_pinned: row.get::<_, i32>(9)? == 1,
                    content_hash: row.get(10)?,
                    session_id: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 按会话获取记录
    pub fn get_items_by_session(
        &self,
        session_id: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, type, content, image_path, thumbnail_path, preview, timestamp,
                    source_app, is_favorite, is_pinned, content_hash, session_id
             FROM clipboard_items
             WHERE session_id = ?1
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let items = stmt
            .query_map(params![session_id, limit, offset], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    type_: ClipboardType::from_str(&row.get::<_, String>(1)?)
                        .unwrap_or(ClipboardType::Text),
                    content: row.get(2)?,
                    image_path: row.get(3)?,
                    thumbnail_path: row.get(4)?,
                    preview: row.get(5)?,
                    timestamp: row.get(6)?,
                    source_app: row.get(7)?,
                    is_favorite: row.get::<_, i32>(8)? == 1,
                    is_pinned: row.get::<_, i32>(9)? == 1,
                    content_hash: row.get(10)?,
                    session_id: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 检查是否存在重复内容（5分钟内）
    pub fn has_duplicate(&self, content_hash: &str, time_window_ms: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp_millis();
        let threshold = now - time_window_ms;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items
             WHERE content_hash = ?1 AND timestamp > ?2",
            params![content_hash, threshold],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// 删除记录
    pub fn delete_item(&self, item_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM clipboard_items WHERE id = ?1",
            params![item_id],
        )?;
        Ok(())
    }

    /// 切换收藏状态
    pub fn toggle_favorite(&self, item_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let is_favorite: i32 = conn.query_row(
            "SELECT is_favorite FROM clipboard_items WHERE id = ?1",
            params![item_id],
            |row| row.get(0),
        )?;

        let new_state = if is_favorite == 1 { 0 } else { 1 };

        conn.execute(
            "UPDATE clipboard_items SET is_favorite = ?1 WHERE id = ?2",
            params![new_state, item_id],
        )?;

        Ok(new_state == 1)
    }

    /// 切换置顶状态
    pub fn toggle_pinned(&self, item_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let is_pinned: i32 = conn.query_row(
            "SELECT is_pinned FROM clipboard_items WHERE id = ?1",
            params![item_id],
            |row| row.get(0),
        )?;

        let new_state = if is_pinned == 1 { 0 } else { 1 };

        conn.execute(
            "UPDATE clipboard_items SET is_pinned = ?1 WHERE id = ?2",
            params![new_state, item_id],
        )?;

        Ok(new_state == 1)
    }

    /// 创建会话
    pub fn create_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp_millis();

        // 结束所有活跃会话
        conn.execute(
            "UPDATE sessions SET is_active = 0, end_time = ?1 WHERE is_active = 1",
            params![now],
        )?;

        // 创建新会话
        conn.execute(
            "INSERT INTO sessions (id, start_time, is_active) VALUES (?1, ?2, 1)",
            params![session_id, now],
        )?;

        Ok(())
    }

    /// 结束会话
    pub fn end_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp_millis();

        // 统计记录数
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        // 更新会话
        conn.execute(
            "UPDATE sessions SET end_time = ?1, item_count = ?2, is_active = 0 WHERE id = ?3",
            params![now, count, session_id],
        )?;

        Ok(())
    }

    /// 获取当前会话
    pub fn get_current_session(&self) -> Result<Option<Session>> {
        let conn = self.conn.lock().unwrap();

        let result = conn.query_row(
            "SELECT id, start_time, end_time, item_count, is_active
             FROM sessions WHERE is_active = 1 LIMIT 1",
            [],
            |row| {
                Ok(Session {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    item_count: row.get(3)?,
                    is_active: row.get::<_, i32>(4)? == 1,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 获取会话列表
    pub fn get_sessions(&self, limit: i32) -> Result<Vec<Session>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, start_time, end_time, item_count, is_active
             FROM sessions
             ORDER BY start_time DESC
             LIMIT ?1",
        )?;

        let sessions = stmt
            .query_map(params![limit], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    item_count: row.get(3)?,
                    is_active: row.get::<_, i32>(4)? == 1,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// 清空会话
    pub fn clear_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 删除该会话的所有记录
        conn.execute(
            "DELETE FROM clipboard_items WHERE session_id = ?1",
            params![session_id],
        )?;

        // 删除会话
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;

        Ok(())
    }

    /// 更新记录内容
    pub fn update_item_content(&self, item_id: &str, new_content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 生成新的预览文本
        let char_count = new_content.chars().count();
        let preview = if char_count > 100 {
            let preview_text: String = new_content.chars().take(100).collect();
            format!("{}...", preview_text)
        } else {
            new_content.to_string()
        };

        conn.execute(
            "UPDATE clipboard_items SET content = ?1, preview = ?2 WHERE id = ?3",
            params![new_content, preview, item_id],
        )?;

        Ok(())
    }

    /// 搜索记录
    pub fn search_items(
        &self,
        query: &str,
        session_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let search_pattern = format!("%{}%", query);

        // 根据是否有 session_id 分别执行不同的查询
        let items = if let Some(sid) = session_id {
            let mut stmt = conn.prepare(
                "SELECT id, type, content, image_path, thumbnail_path, preview, timestamp,
                        source_app, is_favorite, is_pinned, content_hash, session_id
                 FROM clipboard_items
                 WHERE session_id = ?1 AND (content LIKE ?2 OR preview LIKE ?2)
                 ORDER BY timestamp DESC
                 LIMIT ?3",
            )?;

            let rows = stmt.query_map(params![sid, &search_pattern, limit], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    type_: ClipboardType::from_str(&row.get::<_, String>(1)?)
                        .unwrap_or(ClipboardType::Text),
                    content: row.get(2)?,
                    image_path: row.get(3)?,
                    thumbnail_path: row.get(4)?,
                    preview: row.get(5)?,
                    timestamp: row.get(6)?,
                    source_app: row.get(7)?,
                    is_favorite: row.get::<_, i32>(8)? == 1,
                    is_pinned: row.get::<_, i32>(9)? == 1,
                    content_hash: row.get(10)?,
                    session_id: row.get(11)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, type, content, image_path, thumbnail_path, preview, timestamp,
                        source_app, is_favorite, is_pinned, content_hash, session_id
                 FROM clipboard_items
                 WHERE content LIKE ?1 OR preview LIKE ?1
                 ORDER BY timestamp DESC
                 LIMIT ?2",
            )?;

            let rows = stmt.query_map(params![&search_pattern, limit], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    type_: ClipboardType::from_str(&row.get::<_, String>(1)?)
                        .unwrap_or(ClipboardType::Text),
                    content: row.get(2)?,
                    image_path: row.get(3)?,
                    thumbnail_path: row.get(4)?,
                    preview: row.get(5)?,
                    timestamp: row.get(6)?,
                    source_app: row.get(7)?,
                    is_favorite: row.get::<_, i32>(8)? == 1,
                    is_pinned: row.get::<_, i32>(9)? == 1,
                    content_hash: row.get(10)?,
                    session_id: row.get(11)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()?
        };

        Ok(items)
    }
}
