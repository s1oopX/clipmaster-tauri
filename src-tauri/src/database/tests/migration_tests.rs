use super::*;

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
fn records_schema_migrations_for_new_database() {
    let (db, data_dir) = temp_database();

    assert_eq!(migration_versions(&db), vec![1, 2, 3, 4, 5, 6]);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn migrates_legacy_database_schema_and_image_paths() {
    let data_dir = std::env::temp_dir().join(format!("clipmaster-database-{}", nanoid::nanoid!()));
    let timestamp = Utc
        .with_ymd_and_hms(2026, 6, 5, 16, 30, 0)
        .single()
        .unwrap()
        .timestamp_millis();
    create_legacy_database(&data_dir, timestamp);

    let old_image_dir = data_dir.join("images").join("2026-06");
    fs::create_dir_all(&old_image_dir).unwrap();
    fs::write(old_image_dir.join("capture.png"), "legacy-image").unwrap();

    let db = Database::new(data_dir.clone()).unwrap();

    {
        let conn = db.conn.lock().unwrap();
        assert!(column_exists(&conn, "clipboard_items", "thumbnail_path").unwrap());
        assert!(column_exists(&conn, "clipboard_items", "date_key").unwrap());
        assert!(column_exists(&conn, "clipboard_items", "annotation").unwrap());
    }
    assert_eq!(migration_versions(&db), vec![1, 2, 3, 4, 5, 6]);

    let item = db.get_item("legacy_image").unwrap().unwrap();
    assert_eq!(item.date_key, "2026-06-06");
    assert_eq!(
        item.image_path.as_deref(),
        Some("images/2026-06-06/capture.png")
    );
    assert_eq!(item.thumbnail_path, None);
    assert_eq!(item.annotation, None);
    assert!(data_dir
        .join("images")
        .join("2026-06-06")
        .join("capture.png")
        .exists());

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn migrates_single_url_text_records_to_link_type() {
    let data_dir = std::env::temp_dir().join(format!("clipmaster-database-{}", nanoid::nanoid!()));
    fs::create_dir_all(&data_dir).unwrap();
    let timestamp = Utc::now().timestamp_millis();
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
                thumbnail_path TEXT,
                preview TEXT,
                timestamp INTEGER NOT NULL,
                date_key TEXT NOT NULL,
                source_app TEXT,
                is_favorite INTEGER DEFAULT 0,
                is_pinned INTEGER DEFAULT 0,
                annotation TEXT,
                content_hash TEXT NOT NULL,
                session_id TEXT NOT NULL
            );
            CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );",
    )
    .unwrap();
    for version in 1..=5 {
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, ?3)",
            params![version, format!("migration_{version}"), timestamp],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO sessions (id, start_time, item_count, is_active)
             VALUES ('session_1', ?1, 1, 1)",
        params![timestamp],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO clipboard_items (
                id, type, content, image_path, thumbnail_path, preview, timestamp,
                date_key, source_app, is_favorite, is_pinned, annotation, content_hash, session_id
             )
             VALUES (
                'legacy_link', 'text', ' https://example.com/docs ', NULL, NULL,
                ' https://example.com/docs ', ?1, ?2, NULL, 0, 0, NULL, ?3, 'session_1'
             )",
        params![
            timestamp,
            date_key_from_timestamp(timestamp, DEFAULT_TIME_ZONE),
            text_hash(" https://example.com/docs ")
        ],
    )
    .unwrap();
    drop(conn);

    let db = Database::new(data_dir.clone()).unwrap();
    let item = db.get_item("legacy_link").unwrap().unwrap();
    assert!(matches!(item.type_, ClipboardType::Link));
    assert_eq!(item.content.as_deref(), Some("https://example.com/docs"));
    assert_eq!(
        item.content_hash,
        link_content_hash("https://example.com/docs")
    );
    assert_eq!(migration_versions(&db), vec![1, 2, 3, 4, 5, 6]);

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

    let items = db.get_items_filtered(10, 0, None, false).unwrap();
    assert_eq!(items[0].date_key, "2026-06-05");

    let _ = fs::remove_dir_all(data_dir);
}
