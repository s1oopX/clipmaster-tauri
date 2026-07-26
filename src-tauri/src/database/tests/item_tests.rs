use super::*;

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

    let items = db.get_items_filtered(10, 0, None, false).unwrap();
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

    let items = db.get_items_filtered(10, 0, None, false).unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| item.date_key == yesterday_key));

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn updating_text_content_keeps_duplicate_hash_in_sync() {
    let (db, data_dir) = temp_database();
    let alpha_hash = text_hash("Alpha token");
    let beta_hash = text_hash("Beta token");
    let item = db
        .insert_item(text_item(&alpha_hash), DEFAULT_TIME_ZONE)
        .unwrap();

    let updated = db.update_item_content(&item.id, "Beta token").unwrap();
    assert_eq!(updated.content.as_deref(), Some("Beta token"));
    assert_eq!(updated.content_hash, beta_hash);

    let stale_alpha_match = db
        .refresh_duplicate_for_time_zone(&alpha_hash, DEFAULT_TIME_ZONE)
        .unwrap();
    assert!(stale_alpha_match.is_none());

    let beta_match = db
        .refresh_duplicate_for_time_zone(&beta_hash, DEFAULT_TIME_ZONE)
        .unwrap()
        .unwrap();
    assert_eq!(beta_match.id, item.id);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn updating_text_content_to_url_stores_link_type_and_prefixed_hash() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();

    let updated = db
        .update_item_content(&item.id, " https://example.com/docs ")
        .unwrap();

    assert!(matches!(updated.type_, ClipboardType::Link));
    assert_eq!(updated.content.as_deref(), Some("https://example.com/docs"));
    assert_eq!(updated.preview.as_deref(), Some("https://example.com/docs"));
    assert_eq!(
        updated.content_hash,
        link_content_hash("https://example.com/docs")
    );

    let links = db.get_items_filtered(10, 0, Some("link"), false).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].id, item.id);

    let stale_text_match = db
        .refresh_duplicate_for_time_zone(&text_hash("https://example.com/docs"), DEFAULT_TIME_ZONE)
        .unwrap();
    assert!(stale_text_match.is_none());

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn updating_text_content_cannot_create_same_day_duplicate() {
    let (db, data_dir) = temp_database();
    let first = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();
    let second = db
        .insert_item(text_item_with_content("Beta token"), DEFAULT_TIME_ZONE)
        .unwrap();

    let error = db
        .update_item_content(&second.id, "Alpha token")
        .unwrap_err()
        .to_string();
    assert!(error.contains("当日已存在相同内容"));

    let unchanged = db.get_item(&second.id).unwrap().unwrap();
    assert_eq!(unchanged.content.as_deref(), Some("Beta token"));
    assert_eq!(unchanged.content_hash, text_hash("Beta token"));

    let original = db.get_item(&first.id).unwrap().unwrap();
    assert_eq!(original.content.as_deref(), Some("Alpha token"));

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn updating_text_content_rejects_blank_content() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();

    let error = db
        .update_item_content(&item.id, "  \n\t  ")
        .unwrap_err()
        .to_string();
    assert!(error.contains("原文不能为空"));

    let unchanged = db.get_item(&item.id).unwrap().unwrap();
    assert_eq!(unchanged.content.as_deref(), Some("Alpha token"));
    assert_eq!(unchanged.preview.as_deref(), Some("Alpha token"));
    assert_eq!(unchanged.content_hash, text_hash("Alpha token"));

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn updating_annotation_requires_existing_record() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();

    db.update_item_annotation(&item.id, Some("用于发票核对"))
        .unwrap();
    let annotated = db.get_item(&item.id).unwrap().unwrap();
    assert_eq!(annotated.annotation.as_deref(), Some("用于发票核对"));
    assert_eq!(annotated.content.as_deref(), Some("Alpha token"));
    // 标注不再联动收藏：保护由清理逻辑按 annotation 字段直接实现
    assert!(!annotated.is_favorite);

    db.update_item_annotation(&item.id, None).unwrap();
    let cleared = db.get_item(&item.id).unwrap().unwrap();
    assert_eq!(cleared.annotation, None);
    assert!(!cleared.is_favorite);

    let error = db
        .update_item_annotation("missing-item", Some("不会保存"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("记录不存在"));

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn item_state_changes_require_existing_record() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();

    assert!(db.toggle_favorite(&item.id).unwrap());
    assert!(db.toggle_pinned(&item.id).unwrap());
    db.delete_item(&item.id).unwrap();
    assert!(db.get_item(&item.id).unwrap().is_none());

    let delete_error = db.delete_item(&item.id).unwrap_err().to_string();
    assert!(delete_error.contains("记录不存在"));

    let favorite_error = db.toggle_favorite(&item.id).unwrap_err().to_string();
    assert!(favorite_error.contains("记录不存在"));

    let pinned_error = db.toggle_pinned(&item.id).unwrap_err().to_string();
    assert!(pinned_error.contains("记录不存在"));

    let _ = fs::remove_dir_all(data_dir);
}
