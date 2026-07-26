use super::*;

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
        .search_items(
            "Alpha",
            Some("session_1"),
            &today.date_key,
            10,
            0,
            None,
            false,
        )
        .unwrap();
    assert_eq!(today_results.len(), 1);
    assert_eq!(today_results[0].id, today.id);

    let other_day_results = db
        .search_items(
            "Alpha",
            Some("session_1"),
            &other_date_key,
            10,
            0,
            None,
            false,
        )
        .unwrap();
    assert_eq!(other_day_results.len(), 1);
    assert_eq!(other_day_results[0].id, other_day.id);

    let all_session_today_results = db
        .search_items("Alpha", None, &today.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(all_session_today_results.len(), 1);
    assert_eq!(all_session_today_results[0].id, today.id);

    let all_session_other_day_results = db
        .search_items("Alpha", None, &other_date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(all_session_other_day_results.len(), 1);
    assert_eq!(all_session_other_day_results[0].id, other_day.id);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn search_items_matches_cjk_substring_via_fts() {
    let (db, data_dir) = temp_database();
    let hit = db
        .insert_item(
            text_item_with_content("摄影底版调色工具支持语义滑块微调"),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();
    let miss = db
        .insert_item(
            text_item_with_content("完全不相关的另一条记录"),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();

    let results = db
        .search_items("语义滑块", None, &hit.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, hit.id);
    assert_ne!(results[0].id, miss.id);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn search_items_is_case_insensitive_via_fts() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(
            text_item_with_content("Tauri AssetProtocol Scope"),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();

    let results = db
        .search_items("assetprotocol", None, &item.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, item.id);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn search_items_short_query_falls_back_to_like() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("滑块微调"), DEFAULT_TIME_ZONE)
        .unwrap();

    // 两个字符不足以命中 trigram 索引，应回退 LIKE 且仍能找到
    let results = db
        .search_items("滑块", None, &item.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, item.id);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn search_items_fts_index_follows_annotation_update_and_delete() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("普通内容"), DEFAULT_TIME_ZONE)
        .unwrap();

    db.update_item_annotation(&item.id, Some("重要令牌备注"))
        .unwrap();
    let annotated = db
        .search_items("令牌备注", None, &item.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(annotated.len(), 1);
    assert_eq!(annotated[0].id, item.id);

    db.delete_item(&item.id).unwrap();
    let after_delete = db
        .search_items("令牌备注", None, &item.date_key, 10, 0, None, false)
        .unwrap();
    assert!(after_delete.is_empty());

    {
        let conn = db.conn.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO clipboard_items_fts(clipboard_items_fts) VALUES('integrity-check');",
        )
        .unwrap();
    }

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn search_items_treats_empty_and_like_wildcards_as_literals() {
    let (db, data_dir) = temp_database();
    let percent_item = db
        .insert_item(
            text_item_with_content("Progress 100% ready"),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();
    let percent_neighbor = db
        .insert_item(
            text_item_with_content("Progress 100x ready"),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();
    let underscore_item = db
        .insert_item(text_item_with_content("Alpha_token"), DEFAULT_TIME_ZONE)
        .unwrap();
    let underscore_neighbor = db
        .insert_item(text_item_with_content("AlphaXtoken"), DEFAULT_TIME_ZONE)
        .unwrap();

    let empty_results = db
        .search_items("   ", None, &percent_item.date_key, 10, 0, None, false)
        .unwrap();
    assert!(empty_results.is_empty());

    let percent_results = db
        .search_items("%", None, &percent_item.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(percent_results.len(), 1);
    assert_eq!(percent_results[0].id, percent_item.id);
    assert_ne!(percent_results[0].id, percent_neighbor.id);

    let underscore_results = db
        .search_items("_", None, &underscore_item.date_key, 10, 0, None, false)
        .unwrap();
    assert_eq!(underscore_results.len(), 1);
    assert_eq!(underscore_results[0].id, underscore_item.id);
    assert_ne!(underscore_results[0].id, underscore_neighbor.id);

    let _ = fs::remove_dir_all(data_dir);
}
