use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

use crate::link::{link_content_hash, normalize_web_url};
use crate::settings::DEFAULT_TIME_ZONE;

use super::utils::{date_key_from_timestamp, preview_from_content};

pub(super) fn run(conn: &Connection, data_dir: &Path) -> Result<()> {
    run_migration(conn, 1, "add_thumbnail_path", |conn| {
        add_column_if_missing(
            conn,
            "clipboard_items",
            "thumbnail_path",
            "thumbnail_path TEXT",
        )
    })?;
    run_migration(conn, 2, "add_date_key", |conn| {
        add_column_if_missing(conn, "clipboard_items", "date_key", "date_key TEXT")
    })?;
    run_migration(conn, 3, "add_annotation", |conn| {
        add_column_if_missing(conn, "clipboard_items", "annotation", "annotation TEXT")
    })?;
    run_migration(conn, 4, "backfill_date_keys", backfill_date_keys)?;
    run_migration(conn, 5, "migrate_image_paths_to_daily", |conn| {
        migrate_image_paths_to_daily(conn, data_dir)
    })?;
    run_migration(
        conn,
        6,
        "migrate_text_urls_to_links",
        migrate_text_urls_to_links,
    )?;
    run_migration(conn, 7, "add_fts5_trigram_search", add_fts5_trigram_search)?;
    Ok(())
}

/// 建立 trigram 分词的 FTS5 外容表加速子串搜索（含 CJK），并用触发器与主表保持同步。
/// 外容表按 rowid 关联主表；当前代码没有 VACUUM，若未来引入需在其后执行
/// `INSERT INTO clipboard_items_fts(clipboard_items_fts) VALUES('rebuild')`。
fn add_fts5_trigram_search(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_items_fts USING fts5(
             content,
             preview,
             annotation,
             content='clipboard_items',
             content_rowid='rowid',
             tokenize='trigram'
         );
         INSERT INTO clipboard_items_fts(rowid, content, preview, annotation)
             SELECT rowid,
                    coalesce(content, ''),
                    coalesce(preview, ''),
                    coalesce(annotation, '')
             FROM clipboard_items;
         CREATE TRIGGER IF NOT EXISTS clipboard_items_fts_ai
         AFTER INSERT ON clipboard_items BEGIN
             INSERT INTO clipboard_items_fts(rowid, content, preview, annotation)
             VALUES (
                 new.rowid,
                 coalesce(new.content, ''),
                 coalesce(new.preview, ''),
                 coalesce(new.annotation, '')
             );
         END;
         CREATE TRIGGER IF NOT EXISTS clipboard_items_fts_ad
         AFTER DELETE ON clipboard_items BEGIN
             INSERT INTO clipboard_items_fts(
                 clipboard_items_fts, rowid, content, preview, annotation
             )
             VALUES (
                 'delete',
                 old.rowid,
                 coalesce(old.content, ''),
                 coalesce(old.preview, ''),
                 coalesce(old.annotation, '')
             );
         END;
         CREATE TRIGGER IF NOT EXISTS clipboard_items_fts_au
         AFTER UPDATE OF content, preview, annotation ON clipboard_items BEGIN
             INSERT INTO clipboard_items_fts(
                 clipboard_items_fts, rowid, content, preview, annotation
             )
             VALUES (
                 'delete',
                 old.rowid,
                 coalesce(old.content, ''),
                 coalesce(old.preview, ''),
                 coalesce(old.annotation, '')
             );
             INSERT INTO clipboard_items_fts(rowid, content, preview, annotation)
             VALUES (
                 new.rowid,
                 coalesce(new.content, ''),
                 coalesce(new.preview, ''),
                 coalesce(new.annotation, '')
             );
         END;",
    )?;
    Ok(())
}

fn run_migration<F>(conn: &Connection, version: i32, name: &str, migration: F) -> Result<()>
where
    F: FnOnce(&Connection) -> Result<()>,
{
    if migration_applied(conn, version)? {
        return Ok(());
    }

    migration(conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, ?3)",
        params![version, name, Utc::now().timestamp_millis()],
    )?;

    Ok(())
}

fn backfill_date_keys(conn: &Connection) -> Result<()> {
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

fn migrate_image_paths_to_daily(conn: &Connection, data_dir: &Path) -> Result<()> {
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
        let next_image_path = migrate_month_image_path(data_dir, image_path.as_deref(), &date_key)?;
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

fn migrate_text_urls_to_links(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, content
         FROM clipboard_items
         WHERE type = 'text'
           AND content IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let items = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for (id, content) in items {
        let Some(url) = normalize_web_url(&content) else {
            continue;
        };
        conn.execute(
            "UPDATE clipboard_items
             SET type = 'link', content = ?1, preview = ?2, content_hash = ?3
             WHERE id = ?4",
            params![url, preview_from_content(&url), link_content_hash(&url), id],
        )?;
    }

    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    column_definition: &str,
) -> Result<()> {
    if column_exists(conn, table, column)? {
        return Ok(());
    }

    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column_definition}"),
        [],
    )?;

    Ok(())
}

pub(super) fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn migration_applied(conn: &Connection, version: i32) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
        params![version],
        |row| row.get(0),
    )?;
    Ok(count > 0)
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
