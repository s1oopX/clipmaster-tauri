use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::{ClipboardItem, ClipboardType};

#[cfg(test)]
use crate::link::link_content_hash;
#[cfg(test)]
use crate::models::CreateClipboardItem;
#[cfg(test)]
use crate::settings::DEFAULT_TIME_ZONE;

mod cleanup;
mod items;
mod migrations;
mod schema;
mod sessions;
mod utils;

#[cfg(test)]
use chrono::{TimeZone, Utc};
#[cfg(test)]
use migrations::column_exists;
#[cfg(test)]
use utils::date_key_from_timestamp;
pub use utils::date_key_now;

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

        // 创建表结构
        schema::create_tables(&conn)?;
        migrations::run(&conn, &data_dir)?;
        schema::create_indexes(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

/// 为 trigram FTS5 构造子串匹配短语。trigram 索引要求至少 3 个字符才能命中，
/// 更短的查询返回 None，由调用方回退到 LIKE 扫描。
fn fts_phrase_query(query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.chars().count() < 3 {
        return None;
    }

    Some(format!("\"{}\"", trimmed.replace('"', "\"\"")))
}

fn like_literal_pattern(query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut pattern = String::with_capacity(trimmed.len() + 2);
    pattern.push('%');
    for character in trimmed.chars() {
        if matches!(character, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('%');

    Some(pattern)
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

fn session_exists(conn: &Connection, session_id: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE id = ?1",
        params![session_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn refresh_session_item_count(conn: &Connection, session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions
         SET item_count = (
            SELECT COUNT(*)
            FROM clipboard_items
            WHERE session_id = ?1
         )
         WHERE id = ?1",
        params![session_id],
    )?;
    Ok(())
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
mod tests;
