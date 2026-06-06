use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::models::{
    CleanupPlan, ClipboardDay, ClipboardItem, ClipboardType, CreateClipboardItem, Session,
};
use crate::settings::DEFAULT_TIME_ZONE;

pub struct Database {
    conn: Mutex<Connection>,
}

const CLIPBOARD_ITEM_COLUMNS: &str = "\
    id, type, content, image_path, thumbnail_path, preview, timestamp,
    date_key, source_app, is_favorite, is_pinned, content_hash, session_id, annotation";

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
        db.run_migrations(&data_dir)?;

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

        // 迁移：添加 thumbnail_path 字段（如果不存在）
        conn.execute(
            "ALTER TABLE clipboard_items ADD COLUMN thumbnail_path TEXT",
            [],
        )
        .ok(); // 忽略错误，因为字段可能已存在

        // 迁移：添加 date_key 字段（如果不存在）
        conn.execute("ALTER TABLE clipboard_items ADD COLUMN date_key TEXT", [])
            .ok(); // 忽略错误，因为字段可能已存在

        // 迁移：添加 annotation 字段（如果不存在），用于保存不改变原内容的用户标注
        conn.execute("ALTER TABLE clipboard_items ADD COLUMN annotation TEXT", [])
            .ok(); // 忽略错误，因为字段可能已存在

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

    fn run_migrations(&self, data_dir: &Path) -> Result<()> {
        self.backfill_date_keys()?;
        self.migrate_image_paths_to_daily(data_dir)?;
        Ok(())
    }

    fn backfill_date_keys(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp FROM clipboard_items WHERE date_key IS NULL OR date_key = ''",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let items = rows.collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        for (id, timestamp) in items {
            let date_key = date_key_from_timestamp(timestamp, DEFAULT_TIME_ZONE);
            conn.execute(
                "UPDATE clipboard_items SET date_key = ?1 WHERE id = ?2",
                params![date_key, id],
            )?;
        }

        Ok(())
    }

    fn migrate_image_paths_to_daily(&self, data_dir: &Path) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, image_path, thumbnail_path, date_key
             FROM clipboard_items
             WHERE type = 'image'",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let items = rows.collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        for (id, image_path, thumbnail_path, date_key) in items {
            let next_image_path =
                migrate_month_image_path(data_dir, image_path.as_deref(), &date_key)?;
            let next_thumbnail_path =
                migrate_month_image_path(data_dir, thumbnail_path.as_deref(), &date_key)?;

            if next_image_path != image_path || next_thumbnail_path != thumbnail_path {
                conn.execute(
                    "UPDATE clipboard_items SET image_path = ?1, thumbnail_path = ?2 WHERE id = ?3",
                    params![next_image_path, next_thumbnail_path, id],
                )?;
            }
        }

        Ok(())
    }

    pub fn rebuild_date_keys(&self, time_zone: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let items = {
            let mut stmt = tx.prepare("SELECT id, timestamp FROM clipboard_items")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        for (id, timestamp) in items {
            let date_key = date_key_from_timestamp(timestamp, time_zone);
            tx.execute(
                "UPDATE clipboard_items SET date_key = ?1 WHERE id = ?2",
                params![date_key, id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// 插入剪贴板记录；同一设置时区自然日内的相同内容只刷新记录时间。
    pub fn insert_item(&self, item: CreateClipboardItem, time_zone: &str) -> Result<ClipboardItem> {
        let conn = self.conn.lock().unwrap();

        let id = nanoid::nanoid!();
        let timestamp = Utc::now().timestamp_millis();
        let date_key = date_key_from_timestamp(timestamp, time_zone);

        if let Some(existing) =
            refresh_duplicate_for_date(&conn, &item.content_hash, &date_key, timestamp)?
        {
            return Ok(existing);
        }

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
                id, type, content, image_path, thumbnail_path, preview, timestamp, date_key,
                source_app, content_hash, session_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                item.type_.as_str(),
                item.content,
                item.image_path,
                item.thumbnail_path,
                preview,
                timestamp,
                date_key,
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
            date_key,
            source_app: item.source_app,
            is_favorite: false,
            is_pinned: false,
            annotation: None,
            content_hash: item.content_hash,
            session_id: item.session_id,
        })
    }

    /// 获取剪贴板记录列表
    pub fn get_items(&self, limit: i32, offset: i32) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?1 OFFSET ?2",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map(params![limit, offset], clipboard_item_from_row)?
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

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE session_id = ?1
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?2 OFFSET ?3",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map(params![session_id, limit, offset], clipboard_item_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 如果设置时区当天已有相同内容，刷新该记录时间并返回它。
    pub fn refresh_duplicate_for_time_zone(
        &self,
        content_hash: &str,
        time_zone: &str,
    ) -> Result<Option<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let timestamp = Utc::now().timestamp_millis();
        let date_key = date_key_from_timestamp(timestamp, time_zone);

        refresh_duplicate_for_date(&conn, content_hash, &date_key, timestamp)
    }

    /// 获取单条剪贴板记录
    pub fn get_item(&self, item_id: &str) -> Result<Option<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE id = ?1",
            CLIPBOARD_ITEM_COLUMNS
        );
        let result = conn.query_row(&sql, params![item_id], clipboard_item_from_row);

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
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

    /// 更新记录标注，不修改原始内容和预览
    pub fn update_item_annotation(&self, item_id: &str, annotation: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE clipboard_items SET annotation = ?1 WHERE id = ?2",
            params![annotation, item_id],
        )?;

        Ok(())
    }

    /// 获取可用日期列表
    pub fn get_available_days(&self, limit: i32) -> Result<Vec<ClipboardDay>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT date_key, COUNT(*) AS item_count, MIN(timestamp), MAX(timestamp)
             FROM clipboard_items
             WHERE date_key IS NOT NULL AND date_key != ''
             GROUP BY date_key
             ORDER BY date_key DESC
             LIMIT ?1",
        )?;

        let days = stmt
            .query_map(params![limit], |row| {
                Ok(ClipboardDay {
                    date_key: row.get(0)?,
                    item_count: row.get(1)?,
                    start_time: row.get(2)?,
                    end_time: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(days)
    }

    /// 按日期获取记录
    pub fn get_items_by_day(
        &self,
        date_key: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE date_key = ?1
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?2 OFFSET ?3",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map(params![date_key, limit, offset], clipboard_item_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 获取自定义清理候选记录
    pub fn get_cleanup_candidates(
        &self,
        max_items: i32,
        keep_days: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let keep_threshold =
            Utc::now().timestamp_millis() - (keep_days as i64 * 24 * 60 * 60 * 1000);

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE is_pinned = 0
               AND is_favorite = 0
               AND (
                    timestamp < ?2
                    OR id IN (
                        SELECT id FROM (
                            SELECT id
                            FROM clipboard_items
                            WHERE is_pinned = 0 AND is_favorite = 0
                            ORDER BY timestamp DESC
                            LIMIT -1 OFFSET ?1
                        )
                    )
               )
             ORDER BY timestamp ASC",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map(
            params![max_items.max(0), keep_threshold],
            clipboard_item_from_row,
        )?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// 删除多条记录
    pub fn delete_items(&self, item_ids: &[String]) -> Result<()> {
        if item_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        for item_id in item_ids {
            tx.execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![item_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// 预览自定义清理结果
    pub fn cleanup_plan(&self, max_items: i32, keep_days: i32) -> Result<CleanupPlan> {
        let candidates = self.get_cleanup_candidates(max_items, keep_days)?;
        Ok(CleanupPlan::from_items(candidates))
    }

    /// 搜索记录
    pub fn search_items(
        &self,
        query: &str,
        session_id: Option<&str>,
        date_key: &str,
        limit: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let search_pattern = format!("%{}%", query);

        // 根据是否有 session_id 分别执行不同的查询
        let items = if let Some(sid) = session_id {
            let sql = format!(
                "SELECT {}
                 FROM clipboard_items
                 WHERE date_key = ?1
                   AND session_id = ?2
                   AND (content LIKE ?3 OR preview LIKE ?3 OR annotation LIKE ?3)
                 ORDER BY timestamp DESC
                 LIMIT ?4",
                CLIPBOARD_ITEM_COLUMNS
            );
            let mut stmt = conn.prepare(&sql)?;

            let rows = stmt.query_map(
                params![date_key, sid, &search_pattern, limit],
                clipboard_item_from_row,
            )?;

            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            let sql = format!(
                "SELECT {}
                 FROM clipboard_items
                 WHERE date_key = ?1
                   AND (content LIKE ?2 OR preview LIKE ?2 OR annotation LIKE ?2)
                 ORDER BY timestamp DESC
                 LIMIT ?3",
                CLIPBOARD_ITEM_COLUMNS
            );
            let mut stmt = conn.prepare(&sql)?;

            let rows = stmt.query_map(
                params![date_key, &search_pattern, limit],
                clipboard_item_from_row,
            )?;

            rows.collect::<Result<Vec<_>, _>>()?
        };

        Ok(items)
    }
}

fn clipboard_item_from_row(row: &Row<'_>) -> rusqlite::Result<ClipboardItem> {
    Ok(ClipboardItem {
        id: row.get(0)?,
        type_: ClipboardType::from_str(&row.get::<_, String>(1)?).unwrap_or(ClipboardType::Text),
        content: row.get(2)?,
        image_path: row.get(3)?,
        thumbnail_path: row.get(4)?,
        preview: row.get(5)?,
        timestamp: row.get(6)?,
        date_key: row.get(7)?,
        source_app: row.get(8)?,
        is_favorite: row.get::<_, i32>(9)? == 1,
        is_pinned: row.get::<_, i32>(10)? == 1,
        content_hash: row.get(11)?,
        session_id: row.get(12)?,
        annotation: row.get(13)?,
    })
}

pub fn date_key_now(time_zone: &str) -> String {
    date_key_from_timestamp(Utc::now().timestamp_millis(), time_zone)
}

fn date_key_from_timestamp(timestamp: i64, time_zone: &str) -> String {
    let tz = parse_time_zone(time_zone);

    Utc.timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Utc::now)
        .with_timezone(&tz)
        .format("%Y-%m-%d")
        .to_string()
}

fn parse_time_zone(time_zone: &str) -> Tz {
    time_zone
        .parse::<Tz>()
        .ok()
        .or_else(|| DEFAULT_TIME_ZONE.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::Asia::Shanghai)
}

fn refresh_duplicate_for_date(
    conn: &Connection,
    content_hash: &str,
    date_key: &str,
    timestamp: i64,
) -> Result<Option<ClipboardItem>> {
    let existing_id = conn
        .query_row(
            "SELECT id
             FROM clipboard_items
             WHERE content_hash = ?1 AND date_key = ?2
             ORDER BY timestamp DESC
             LIMIT 1",
            params![content_hash, date_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    let Some(existing_id) = existing_id else {
        return Ok(None);
    };

    conn.execute(
        "UPDATE clipboard_items SET timestamp = ?1 WHERE id = ?2",
        params![timestamp, existing_id],
    )?;

    let sql = format!(
        "SELECT {}
         FROM clipboard_items
         WHERE id = ?1",
        CLIPBOARD_ITEM_COLUMNS
    );
    let item = conn.query_row(&sql, params![existing_id], clipboard_item_from_row)?;

    Ok(Some(item))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::{fs, path::PathBuf, time::Duration};

    fn temp_database() -> (Database, PathBuf) {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-database-{}", nanoid::nanoid!()));
        let db = Database::new(data_dir.clone()).unwrap();
        db.create_session("session_1").unwrap();
        (db, data_dir)
    }

    fn text_item(content_hash: &str) -> CreateClipboardItem {
        CreateClipboardItem {
            type_: ClipboardType::Text,
            content: Some("Alpha token".to_string()),
            image_path: None,
            thumbnail_path: None,
            source_app: None,
            content_hash: content_hash.to_string(),
            session_id: "session_1".to_string(),
        }
    }

    #[test]
    fn date_keys_follow_configured_time_zone() {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 6, 5, 16, 30, 0)
            .single()
            .unwrap()
            .timestamp_millis();

        assert_eq!(
            date_key_from_timestamp(timestamp, "Asia/Shanghai"),
            "2026-06-06"
        );
        assert_eq!(
            date_key_from_timestamp(timestamp, "America/New_York"),
            "2026-06-05"
        );
    }

    #[test]
    fn refreshes_duplicate_content_within_same_beijing_day() {
        let (db, data_dir) = temp_database();

        let first = db
            .insert_item(text_item("same_hash"), DEFAULT_TIME_ZONE)
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let refreshed = db
            .insert_item(text_item("same_hash"), DEFAULT_TIME_ZONE)
            .unwrap();

        assert_eq!(first.id, refreshed.id);
        assert_eq!(refreshed.date_key, date_key_now(DEFAULT_TIME_ZONE));
        assert!(refreshed.timestamp > first.timestamp);

        let items = db.get_items(10, 0).unwrap();
        assert_eq!(items.len(), 1);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn stores_same_content_again_on_a_new_beijing_day() {
        let (db, data_dir) = temp_database();

        let first = db
            .insert_item(text_item("same_hash"), DEFAULT_TIME_ZONE)
            .unwrap();
        let yesterday_timestamp = Utc::now().timestamp_millis() - 24 * 60 * 60 * 1000;
        let yesterday_key = date_key_from_timestamp(yesterday_timestamp, DEFAULT_TIME_ZONE);

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE clipboard_items SET timestamp = ?1, date_key = ?2 WHERE id = ?3",
                params![yesterday_timestamp, yesterday_key, first.id],
            )
            .unwrap();
        }

        let second = db
            .insert_item(text_item("same_hash"), DEFAULT_TIME_ZONE)
            .unwrap();

        assert_ne!(first.id, second.id);
        assert_eq!(second.date_key, date_key_now(DEFAULT_TIME_ZONE));

        let items = db.get_items(10, 0).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item.date_key == yesterday_key));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn rebuilds_existing_date_keys_for_selected_time_zone() {
        let (db, data_dir) = temp_database();
        let item = db
            .insert_item(text_item("same_hash"), DEFAULT_TIME_ZONE)
            .unwrap();
        let timestamp = Utc
            .with_ymd_and_hms(2026, 6, 5, 16, 30, 0)
            .single()
            .unwrap()
            .timestamp_millis();

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE clipboard_items SET timestamp = ?1, date_key = '2026-06-06' WHERE id = ?2",
                params![timestamp, item.id],
            )
            .unwrap();
        }

        db.rebuild_date_keys("America/New_York").unwrap();

        let items = db.get_items(10, 0).unwrap();
        assert_eq!(items[0].date_key, "2026-06-05");

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn search_items_only_returns_requested_date_key() {
        let (db, data_dir) = temp_database();
        let today = db
            .insert_item(text_item("today_hash"), DEFAULT_TIME_ZONE)
            .unwrap();
        let other_day = db
            .insert_item(text_item("other_day_hash"), DEFAULT_TIME_ZONE)
            .unwrap();
        let other_date_key = if today.date_key == "2026-06-06" {
            "2026-06-05".to_string()
        } else {
            "2026-06-06".to_string()
        };

        {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE clipboard_items SET date_key = ?1 WHERE id = ?2",
                params![other_date_key, other_day.id],
            )
            .unwrap();
        }

        let today_results = db
            .search_items("Alpha", Some("session_1"), &today.date_key, 10)
            .unwrap();
        assert_eq!(today_results.len(), 1);
        assert_eq!(today_results[0].id, today.id);

        let other_day_results = db
            .search_items("Alpha", Some("session_1"), &other_date_key, 10)
            .unwrap();
        assert_eq!(other_day_results.len(), 1);
        assert_eq!(other_day_results[0].id, other_day.id);

        let _ = fs::remove_dir_all(data_dir);
    }
}

fn migrate_month_image_path(
    data_dir: &Path,
    relative_path: Option<&str>,
    date_key: &str,
) -> Result<Option<String>> {
    let Some(relative_path) = relative_path else {
        return Ok(None);
    };

    let normalized = relative_path.replace('\\', "/");
    let parts = normalized.split('/').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "images" || !is_month_key(parts[1]) {
        return Ok(Some(normalized));
    }

    let filename = parts[2];
    let target_relative = format!("images/{}/{}", date_key, filename);
    let source_path = data_dir.join(path_from_forward_slashes(&normalized));
    let target_path = data_dir.join(path_from_forward_slashes(&target_relative));

    if target_path.exists() {
        return Ok(Some(target_relative));
    }

    if !source_path.exists() {
        return Ok(Some(normalized));
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&source_path, &target_path)?;

    if target_path.exists() {
        Ok(Some(target_relative))
    } else {
        Ok(Some(normalized))
    }
}

fn is_month_key(value: &str) -> bool {
    value.len() == 7
        && value.as_bytes()[4] == b'-'
        && value[..4].chars().all(|c| c.is_ascii_digit())
        && value[5..].chars().all(|c| c.is_ascii_digit())
}

fn path_from_forward_slashes(path: &str) -> PathBuf {
    let mut path_buf = PathBuf::new();
    for part in path.split('/') {
        path_buf.push(part);
    }
    path_buf
}
