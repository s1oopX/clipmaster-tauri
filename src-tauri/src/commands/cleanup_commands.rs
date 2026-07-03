use std::path::Path;
use tauri::{AppHandle, State};

use crate::app_data;
use crate::database::Database;
use crate::models::{CleanupFileTarget, CleanupPlan, ClipboardItem};

use super::image_assets::remove_app_data_file_in_dir;

/// 预览自定义清理结果
#[tauri::command]
pub async fn preview_custom_cleanup(
    db: State<'_, Database>,
    max_items: i32,
    keep_days: i32,
) -> Result<CleanupPlan, String> {
    let max_items = max_items.clamp(10, 5000);
    let keep_days = keep_days.clamp(1, 3650);
    db.cleanup_plan(max_items, keep_days)
        .map_err(|e| e.to_string())
}

/// 执行自定义清理
#[tauri::command]
pub async fn run_custom_cleanup(
    app: AppHandle,
    db: State<'_, Database>,
    max_items: i32,
    keep_days: i32,
) -> Result<CleanupPlan, String> {
    let max_items = max_items.clamp(10, 5000);
    let keep_days = keep_days.clamp(1, 3650);
    run_cleanup(&app, &db, max_items, keep_days)
}

/// 清空全部剪贴板历史，包括收藏、置顶、标注记录和图片文件。
#[tauri::command]
pub async fn clear_all_history(
    app: AppHandle,
    db: State<'_, Database>,
) -> Result<CleanupPlan, String> {
    let (plan, file_targets) = db.clear_all_history().map_err(|e| e.to_string())?;

    for target in &file_targets {
        cleanup_file_target_best_effort(&app, target);
    }

    Ok(plan)
}

fn cleanup_item_files(app: &AppHandle, item: &ClipboardItem) -> Result<(), String> {
    let Some(target) = CleanupFileTarget::from_item(item) else {
        return Ok(());
    };
    cleanup_file_target(app, &target)
}

fn cleanup_file_target(app: &AppHandle, target: &CleanupFileTarget) -> Result<(), String> {
    let app_data_dir = app_data::resolve_app_data_dir(app).map_err(|e| e.to_string())?;
    cleanup_file_target_in_dir(&app_data_dir, target)
}

pub(super) fn cleanup_file_target_in_dir(
    app_data_dir: &Path,
    target: &CleanupFileTarget,
) -> Result<(), String> {
    if let Some(path) = &target.image_path {
        remove_app_data_file_in_dir(app_data_dir, path)?;
    }

    if let Some(path) = &target.thumbnail_path {
        remove_app_data_file_in_dir(app_data_dir, path)?;
    }

    Ok(())
}

fn run_cleanup(
    app: &AppHandle,
    db: &Database,
    max_items: i32,
    keep_days: i32,
) -> Result<CleanupPlan, String> {
    let items = db
        .get_cleanup_candidates(max_items, keep_days)
        .map_err(|e| e.to_string())?;
    let plan = CleanupPlan::from_items(items.clone());
    let item_ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();

    db.delete_items(&item_ids).map_err(|e| e.to_string())?;

    for item in &items {
        cleanup_item_files_best_effort(app, item);
    }

    Ok(plan)
}

pub(super) fn cleanup_item_files_best_effort(app: &AppHandle, item: &ClipboardItem) {
    if let Err(error) = cleanup_item_files(app, item) {
        eprintln!("清理记录文件失败（{}）: {}", item.id, error);
    }
}

pub(super) fn cleanup_file_target_best_effort(app: &AppHandle, target: &CleanupFileTarget) {
    if let Err(error) = cleanup_file_target(app, target) {
        eprintln!("清理记录文件失败（{}）: {}", target.id, error);
    }
}
