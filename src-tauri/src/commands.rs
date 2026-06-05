use tauri::State;

use crate::database::Database;
use crate::models::{ClipboardItem, Session};

/// 获取剪贴板记录列表
#[tauri::command]
pub async fn get_clipboard_items(
    db: State<'_, Database>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items(limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

/// 按会话获取记录
#[tauri::command]
pub async fn get_items_by_session(
    db: State<'_, Database>,
    session_id: String,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.get_items_by_session(&session_id, limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

/// 删除记录
#[tauri::command]
pub async fn delete_item(db: State<'_, Database>, item_id: String) -> Result<(), String> {
    db.delete_item(&item_id).map_err(|e| e.to_string())
}

/// 切换收藏状态
#[tauri::command]
pub async fn toggle_favorite(
    db: State<'_, Database>,
    item_id: String,
) -> Result<bool, String> {
    db.toggle_favorite(&item_id).map_err(|e| e.to_string())
}

/// 切换置顶状态
#[tauri::command]
pub async fn toggle_pinned(db: State<'_, Database>, item_id: String) -> Result<bool, String> {
    db.toggle_pinned(&item_id).map_err(|e| e.to_string())
}

/// 获取当前会话
#[tauri::command]
pub async fn get_current_session(
    db: State<'_, Database>,
) -> Result<Option<Session>, String> {
    db.get_current_session().map_err(|e| e.to_string())
}

/// 获取会话列表
#[tauri::command]
pub async fn get_sessions(
    db: State<'_, Database>,
    limit: Option<i32>,
) -> Result<Vec<Session>, String> {
    db.get_sessions(limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

/// 清空会话
#[tauri::command]
pub async fn clear_session(db: State<'_, Database>, session_id: String) -> Result<(), String> {
    db.clear_session(&session_id).map_err(|e| e.to_string())
}

/// 搜索记录
#[tauri::command]
pub async fn search_items(
    db: State<'_, Database>,
    query: String,
    session_id: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ClipboardItem>, String> {
    db.search_items(&query, session_id.as_deref(), limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}
