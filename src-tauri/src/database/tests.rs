use super::*;
use rusqlite::{params, Connection};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

fn temp_database() -> (Database, PathBuf) {
    let data_dir = std::env::temp_dir().join(format!("clipmaster-database-{}", nanoid::nanoid!()));
    let db = Database::new(data_dir.clone()).unwrap();
    db.create_session("session_1").unwrap();
    (db, data_dir)
}

fn migration_versions(db: &Database) -> Vec<i32> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .unwrap();
    stmt.query_map([], |row| row.get::<_, i32>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn create_legacy_database(data_dir: &Path, timestamp: i64) {
    fs::create_dir_all(data_dir).unwrap();
    let conn = Connection::open(data_dir.join("clipboard.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                item_count INTEGER DEFAULT 0,
                is_active INTEGER DEFAULT 1
            );
            CREATE TABLE clipboard_items (
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
            );",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO sessions (id, start_time, item_count, is_active)
             VALUES ('session_1', ?1, 1, 1)",
        params![timestamp],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO clipboard_items (
                id, type, content, image_path, preview, timestamp, source_app,
                is_favorite, is_pinned, content_hash, session_id
             )
             VALUES (
                'legacy_image', 'image', NULL, 'images/2026-06/capture.png', '图片',
                ?1, NULL, 0, 0, 'legacy_hash', 'session_1'
             )",
        params![timestamp],
    )
    .unwrap();
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

fn text_hash(content: &str) -> String {
    format!("{:x}", md5::compute(content.as_bytes()))
}

fn text_item_with_content(content: &str) -> CreateClipboardItem {
    CreateClipboardItem {
        type_: ClipboardType::Text,
        content: Some(content.to_string()),
        image_path: None,
        thumbnail_path: None,
        source_app: None,
        content_hash: text_hash(content),
        session_id: "session_1".to_string(),
    }
}

fn image_item_with_paths(
    image_path: &str,
    thumbnail_path: &str,
    session_id: &str,
) -> CreateClipboardItem {
    CreateClipboardItem {
        type_: ClipboardType::Image,
        content: None,
        image_path: Some(image_path.to_string()),
        thumbnail_path: Some(thumbnail_path.to_string()),
        source_app: None,
        content_hash: text_hash(image_path),
        session_id: session_id.to_string(),
    }
}

mod item_tests;
mod migration_tests;
mod search_tests;
mod session_cleanup_tests;
