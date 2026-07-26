use std::fs;
use std::path::Path;

use anyhow::Result;
use tauri::{AppHandle, Manager};

use crate::database::Database;

pub struct SweepSummary {
    pub removed_images: usize,
    pub removed_cache_files: usize,
}

/// 启动后台清扫：删除数据库不再引用的图片文件与上次运行遗留的冻结截图缓存。
///
/// 记录删除采用「先删库、后 best-effort 删文件」策略，文件删除失败会遗留孤儿；
/// 截图流程中途崩溃也会在 screenshot-cache 留下 freeze_*.png。两者只能靠启动清扫回收。
pub fn sweep_orphan_files(app_data_dir: &Path, app: &AppHandle) -> Result<SweepSummary> {
    let db = app.state::<Database>();
    sweep_orphan_files_in_dir(app_data_dir, &db)
}

pub fn sweep_orphan_files_in_dir(app_data_dir: &Path, db: &Database) -> Result<SweepSummary> {
    let referenced = db.referenced_image_paths()?;
    let mut removed_images = 0usize;

    let images_dir = app_data_dir.join("images");
    if images_dir.is_dir() {
        for date_entry in fs::read_dir(&images_dir)? {
            let date_path = date_entry?.path();
            if !date_path.is_dir() {
                continue;
            }
            let Some(date_name) = date_path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };

            for file_entry in fs::read_dir(&date_path)? {
                let file_path = file_entry?.path();
                if !file_path.is_file() {
                    continue;
                }
                let Some(file_name) = file_path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };

                let relative = format!("images/{}/{}", date_name, file_name);
                if !referenced.contains(&relative) && fs::remove_file(&file_path).is_ok() {
                    removed_images += 1;
                }
            }

            // 目录为空时顺带移除（非空会失败，忽略即可）
            let _ = fs::remove_dir(&date_path);
        }
    }

    // 启动时不存在进行中的截图流程，冻结缓存可以整目录回收
    let mut removed_cache_files = 0usize;
    let cache_dir = app_data_dir.join("screenshot-cache");
    if cache_dir.is_dir() {
        for entry in fs::read_dir(&cache_dir)? {
            let path = entry?.path();
            if path.is_file() && fs::remove_file(&path).is_ok() {
                removed_cache_files += 1;
            }
        }
    }

    Ok(SweepSummary {
        removed_images,
        removed_cache_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ClipboardType, CreateClipboardItem};
    use crate::settings::DEFAULT_TIME_ZONE;

    #[test]
    fn sweep_removes_orphans_and_keeps_referenced_files() {
        let data_dir =
            std::env::temp_dir().join(format!("clipmaster-maintenance-{}", nanoid::nanoid!()));
        let db = Database::new(data_dir.clone()).unwrap();
        db.create_session("session_1").unwrap();

        let saved = db
            .insert_item(
                CreateClipboardItem {
                    type_: ClipboardType::Image,
                    content: None,
                    image_path: Some("images/2026-07-26/keep.png".to_string()),
                    thumbnail_path: Some("images/2026-07-26/keep_thumb.png".to_string()),
                    source_app: None,
                    content_hash: "keep_hash".to_string(),
                    session_id: "session_1".to_string(),
                },
                DEFAULT_TIME_ZONE,
            )
            .unwrap();

        let date_dir = data_dir.join("images").join("2026-07-26");
        fs::create_dir_all(&date_dir).unwrap();
        fs::write(date_dir.join("keep.png"), b"png").unwrap();
        fs::write(date_dir.join("keep_thumb.png"), b"png").unwrap();
        fs::write(date_dir.join("orphan.png"), b"png").unwrap();

        let empty_date_dir = data_dir.join("images").join("2026-07-01");
        fs::create_dir_all(&empty_date_dir).unwrap();
        fs::write(empty_date_dir.join("stale.png"), b"png").unwrap();

        let cache_dir = data_dir.join("screenshot-cache");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("freeze_stale.png"), b"png").unwrap();

        let summary = sweep_orphan_files_in_dir(&data_dir, &db).unwrap();

        assert_eq!(summary.removed_images, 2);
        assert_eq!(summary.removed_cache_files, 1);
        assert!(date_dir.join("keep.png").is_file());
        assert!(date_dir.join("keep_thumb.png").is_file());
        assert!(!date_dir.join("orphan.png").exists());
        assert!(!empty_date_dir.exists());
        assert!(!cache_dir.join("freeze_stale.png").exists());
        assert_eq!(
            saved.image_path.as_deref(),
            Some("images/2026-07-26/keep.png")
        );

        let _ = fs::remove_dir_all(data_dir);
    }
}
