use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[cfg(test)]
const CURRENT_APP_IDENTIFIER: &str = "com.clipmaster.desktop";
pub const LEGACY_APP_IDENTIFIER: &str = "com.clipmaster.app";
pub const APP_DATA_DIR_ENV: &str = "CLIPMASTER_APP_DATA_DIR";

pub fn resolve_app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    if let Some(path) = configured_app_data_dir() {
        fs::create_dir_all(&path).with_context(|| {
            format!("Failed to create configured app data directory {:?}", path)
        })?;
        return Ok(path);
    }

    app.path()
        .app_data_dir()
        .context("Failed to get app data dir")
}

fn configured_app_data_dir() -> Option<PathBuf> {
    std::env::var_os(APP_DATA_DIR_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

pub fn migrate_legacy_app_data_dir(current_dir: &Path) -> Result<()> {
    let Some(parent_dir) = current_dir.parent() else {
        return Ok(());
    };
    let legacy_dir = parent_dir.join(LEGACY_APP_IDENTIFIER);

    if legacy_dir == current_dir || !legacy_dir.exists() {
        return Ok(());
    }

    if !legacy_dir.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(parent_dir).context("Failed to create app data parent directory")?;

    if !current_dir.exists() {
        fs::rename(&legacy_dir, current_dir)
            .with_context(|| migration_context(&legacy_dir, current_dir))?;
        return Ok(());
    }

    if is_dir_empty(current_dir)? {
        fs::remove_dir(current_dir).with_context(|| {
            format!(
                "Failed to remove empty app data directory {:?}",
                current_dir
            )
        })?;
        fs::rename(&legacy_dir, current_dir)
            .with_context(|| migration_context(&legacy_dir, current_dir))?;
        return Ok(());
    }

    merge_missing_entries(&legacy_dir, current_dir)?;
    let _ = fs::remove_dir(&legacy_dir);

    Ok(())
}

fn migration_context(legacy_dir: &Path, current_dir: &Path) -> String {
    format!(
        "Failed to migrate legacy app data directory from {:?} to {:?}",
        legacy_dir, current_dir
    )
}

fn is_dir_empty(path: &Path) -> Result<bool> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read app data directory {:?}", path))?;
    Ok(entries.next().is_none())
}

fn merge_missing_entries(source_dir: &Path, target_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_dir)
        .with_context(|| format!("Failed to create app data directory {:?}", target_dir))?;

    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("Failed to read legacy app data directory {:?}", source_dir))?
    {
        let entry =
            entry.with_context(|| format!("Failed to read entry under {:?}", source_dir))?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());

        if !target_path.exists() {
            fs::rename(&source_path, &target_path).with_context(|| {
                format!(
                    "Failed to move legacy app data entry from {:?} to {:?}",
                    source_path, target_path
                )
            })?;
            continue;
        }

        if source_path.is_dir() && target_path.is_dir() {
            merge_missing_entries(&source_path, &target_path)?;
            let _ = fs::remove_dir(&source_path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_parent() -> PathBuf {
        std::env::temp_dir().join(format!("clipmaster-app-data-{}", nanoid::nanoid!()))
    }

    #[test]
    fn reads_configured_app_data_dir_from_environment() {
        let _guard = ENV_LOCK.lock().unwrap();
        let previous = std::env::var_os(APP_DATA_DIR_ENV);
        let data_dir = temp_parent().join("custom-data");

        std::env::set_var(APP_DATA_DIR_ENV, &data_dir);
        assert_eq!(configured_app_data_dir(), Some(data_dir));

        match previous {
            Some(value) => std::env::set_var(APP_DATA_DIR_ENV, value),
            None => std::env::remove_var(APP_DATA_DIR_ENV),
        }
    }

    #[test]
    fn renames_legacy_data_dir_when_new_dir_is_missing() {
        let parent = temp_parent();
        let legacy_dir = parent.join(LEGACY_APP_IDENTIFIER);
        let current_dir = parent.join(CURRENT_APP_IDENTIFIER);
        fs::create_dir_all(legacy_dir.join("images").join("2026-06-07")).unwrap();
        fs::write(legacy_dir.join("clipboard.db"), "legacy-db").unwrap();
        fs::write(
            legacy_dir
                .join("images")
                .join("2026-06-07")
                .join("capture.png"),
            "legacy-image",
        )
        .unwrap();

        migrate_legacy_app_data_dir(&current_dir).unwrap();

        assert!(!legacy_dir.exists());
        assert_eq!(
            fs::read_to_string(current_dir.join("clipboard.db")).unwrap(),
            "legacy-db"
        );
        assert_eq!(
            fs::read_to_string(
                current_dir
                    .join("images")
                    .join("2026-06-07")
                    .join("capture.png")
            )
            .unwrap(),
            "legacy-image"
        );

        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn replaces_empty_current_dir_with_legacy_data_dir() {
        let parent = temp_parent();
        let legacy_dir = parent.join(LEGACY_APP_IDENTIFIER);
        let current_dir = parent.join(CURRENT_APP_IDENTIFIER);
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::create_dir_all(&current_dir).unwrap();
        fs::write(legacy_dir.join("settings.json"), "legacy-settings").unwrap();

        migrate_legacy_app_data_dir(&current_dir).unwrap();

        assert!(!legacy_dir.exists());
        assert_eq!(
            fs::read_to_string(current_dir.join("settings.json")).unwrap(),
            "legacy-settings"
        );

        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn merges_legacy_files_without_overwriting_current_data() {
        let parent = temp_parent();
        let legacy_dir = parent.join(LEGACY_APP_IDENTIFIER);
        let current_dir = parent.join(CURRENT_APP_IDENTIFIER);
        fs::create_dir_all(legacy_dir.join("images").join("2026-06-07")).unwrap();
        fs::create_dir_all(current_dir.join("images").join("2026-06-07")).unwrap();
        fs::write(legacy_dir.join("settings.json"), "legacy-settings").unwrap();
        fs::write(current_dir.join("settings.json"), "current-settings").unwrap();
        fs::write(
            legacy_dir
                .join("images")
                .join("2026-06-07")
                .join("legacy.png"),
            "legacy-image",
        )
        .unwrap();
        fs::write(
            current_dir
                .join("images")
                .join("2026-06-07")
                .join("current.png"),
            "current-image",
        )
        .unwrap();

        migrate_legacy_app_data_dir(&current_dir).unwrap();

        assert_eq!(
            fs::read_to_string(current_dir.join("settings.json")).unwrap(),
            "current-settings"
        );
        assert_eq!(
            fs::read_to_string(legacy_dir.join("settings.json")).unwrap(),
            "legacy-settings"
        );
        assert_eq!(
            fs::read_to_string(
                current_dir
                    .join("images")
                    .join("2026-06-07")
                    .join("legacy.png")
            )
            .unwrap(),
            "legacy-image"
        );
        assert_eq!(
            fs::read_to_string(
                current_dir
                    .join("images")
                    .join("2026-06-07")
                    .join("current.png")
            )
            .unwrap(),
            "current-image"
        );

        let _ = fs::remove_dir_all(parent);
    }
}
