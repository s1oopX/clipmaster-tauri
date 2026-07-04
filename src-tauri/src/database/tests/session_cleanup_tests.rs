use super::*;

#[test]
fn session_item_counts_follow_inserts_and_deletes() {
    let (db, data_dir) = temp_database();
    assert_eq!(db.get_current_session().unwrap().unwrap().item_count, 0);

    let first = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();
    assert_eq!(db.get_current_session().unwrap().unwrap().item_count, 1);

    let duplicate = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();
    assert_eq!(duplicate.id, first.id);
    assert_eq!(db.get_current_session().unwrap().unwrap().item_count, 1);

    let second = db
        .insert_item(text_item_with_content("Beta token"), DEFAULT_TIME_ZONE)
        .unwrap();
    assert_eq!(db.get_current_session().unwrap().unwrap().item_count, 2);

    db.create_session("session_2").unwrap();
    let sessions = db.get_sessions(10).unwrap();
    let first_session = sessions
        .iter()
        .find(|session| session.id == "session_1")
        .unwrap();
    assert!(!first_session.is_active);
    assert_eq!(first_session.item_count, 2);

    db.delete_item(&first.id).unwrap();
    let sessions = db.get_sessions(10).unwrap();
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.id == "session_1")
            .unwrap()
            .item_count,
        1
    );

    db.delete_items(&[second.id]).unwrap();
    let sessions = db.get_sessions(10).unwrap();
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.id == "session_1")
            .unwrap()
            .item_count,
        0
    );

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn clearing_all_history_removes_records_and_preserves_active_session() {
    let (db, data_dir) = temp_database();
    db.insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();
    let first_image = db
        .insert_item(
            image_item_with_paths(
                "images/2026-06-09/first.png",
                "images/2026-06-09/first-thumb.png",
                "session_1",
            ),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();

    db.create_session("session_2").unwrap();
    db.insert_item(
        CreateClipboardItem {
            session_id: "session_2".to_string(),
            ..text_item_with_content("Beta token")
        },
        DEFAULT_TIME_ZONE,
    )
    .unwrap();
    let session_image = db
        .insert_item(
            image_item_with_paths(
                "images/2026-06-09/session.png",
                "images/2026-06-09/session-thumb.png",
                "session_2",
            ),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();

    let (plan, file_targets) = db.clear_all_history().unwrap();
    let file_target_ids = file_targets
        .iter()
        .map(|target| target.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(plan.item_count, 4);
    assert_eq!(plan.text_count, 2);
    assert_eq!(plan.image_count, 2);
    assert!(plan.oldest_timestamp.is_some());
    assert!(plan.newest_timestamp.is_some());
    assert_eq!(file_targets.len(), 2);
    assert!(file_target_ids.contains(&session_image.id.as_str()));
    assert!(file_target_ids.contains(&first_image.id.as_str()));
    let session_target = file_targets
        .iter()
        .find(|target| target.id == session_image.id)
        .unwrap();
    assert_eq!(
        session_target.image_path.as_deref(),
        Some("images/2026-06-09/session.png")
    );
    assert_eq!(
        session_target.thumbnail_path.as_deref(),
        Some("images/2026-06-09/session-thumb.png")
    );
    assert!(db
        .get_items_filtered(10, 0, None, false)
        .unwrap()
        .is_empty());

    let sessions = db.get_sessions(10).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "session_2");
    assert!(sessions[0].is_active);
    assert_eq!(sessions[0].item_count, 0);

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn inserting_item_requires_existing_session() {
    let (db, data_dir) = temp_database();

    let error = db
        .insert_item(
            CreateClipboardItem {
                session_id: "missing-session".to_string(),
                ..text_item_with_content("Alpha token")
            },
            DEFAULT_TIME_ZONE,
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("会话不存在"));
    assert!(db
        .get_items_filtered(10, 0, None, false)
        .unwrap()
        .is_empty());

    let _ = fs::remove_dir_all(data_dir);
}

#[test]
fn clearing_session_requires_existing_session_and_removes_records() {
    let (db, data_dir) = temp_database();
    let item = db
        .insert_item(text_item_with_content("Alpha token"), DEFAULT_TIME_ZONE)
        .unwrap();
    let image = db
        .insert_item(
            image_item_with_paths(
                "images/2026-06-09/capture.png",
                "images/2026-06-09/thumb.png",
                "session_1",
            ),
            DEFAULT_TIME_ZONE,
        )
        .unwrap();

    let file_targets = db.clear_session("session_1").unwrap();
    assert_eq!(file_targets.len(), 1);
    assert_eq!(file_targets[0].id, image.id);
    assert_eq!(
        file_targets[0].image_path.as_deref(),
        Some("images/2026-06-09/capture.png")
    );
    assert!(db.get_item(&item.id).unwrap().is_none());
    assert!(!db
        .get_sessions(10)
        .unwrap()
        .iter()
        .any(|session| session.id == "session_1"));

    let missing_error = db.clear_session("session_1").unwrap_err().to_string();
    assert!(missing_error.contains("会话不存在"));

    db.create_session("empty_session").unwrap();
    db.clear_session("empty_session").unwrap();
    assert!(!db
        .get_sessions(10)
        .unwrap()
        .iter()
        .any(|session| session.id == "empty_session"));

    let _ = fs::remove_dir_all(data_dir);
}
