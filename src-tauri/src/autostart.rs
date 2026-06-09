use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

const APP_NAME: &str = "ClipMaster";

/// 获取应用程序可执行文件的路径
fn get_exe_path(_app: &AppHandle) -> Result<PathBuf> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    Ok(exe_path)
}

/// 启用开机自启动
pub fn enable_autostart(app: &AppHandle) -> Result<()> {
    let exe_path = get_exe_path(app)?;
    platform_enable_autostart(&exe_path)
}

/// 禁用开机自启动
pub fn disable_autostart(_app: &AppHandle) -> Result<()> {
    platform_disable_autostart()
}

/// 检查是否已启用开机自启动
#[allow(dead_code)]
pub fn is_autostart_enabled(_app: &AppHandle) -> Result<bool> {
    platform_is_autostart_enabled()
}

#[cfg(target_os = "windows")]
fn platform_enable_autostart(exe_path: &Path) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
        .context("Failed to open Windows Run registry key")?;

    let exe_path_str = exe_path
        .to_str()
        .context("Failed to convert exe path to string")?;

    // 使用引号包裹路径，避免空格问题
    let value = format!("\"{}\"", exe_path_str);

    run_key
        .set_value(APP_NAME, &value)
        .context("Failed to set autostart registry value")?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn platform_disable_autostart() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
        .context("Failed to open Windows Run registry key")?;

    // 删除注册表项，如果不存在也不报错
    match run_key.delete_value(APP_NAME) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("Failed to delete autostart registry value"),
    }
}

#[cfg(target_os = "windows")]
fn platform_is_autostart_enabled() -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .context("Failed to open Windows Run registry key")?;

    match run_key.get_value::<String, _>(APP_NAME) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).context("Failed to read autostart registry value"),
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_enable_autostart(_exe_path: &PathBuf) -> Result<()> {
    Err(anyhow::anyhow!("Autostart is only supported on Windows"))
}

#[cfg(not(target_os = "windows"))]
fn platform_disable_autostart() -> Result<()> {
    Err(anyhow::anyhow!("Autostart is only supported on Windows"))
}

#[cfg(not(target_os = "windows"))]
fn platform_is_autostart_enabled() -> Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    #[ignore = "modifies the Windows Run registry key"]
    fn autostart_registry_operations() {
        // 这个测试会实际修改注册表，所以只在需要时手动运行
        // 确保清理状态
        let _ = platform_disable_autostart();

        // 测试启用
        let test_path = PathBuf::from(r"C:\Program Files\ClipMaster\clipmaster.exe");
        assert!(platform_enable_autostart(&test_path).is_ok());
        assert!(platform_is_autostart_enabled().unwrap());

        // 测试禁用
        assert!(platform_disable_autostart().is_ok());
        assert!(!platform_is_autostart_enabled().unwrap());

        // 重复禁用不应报错
        assert!(platform_disable_autostart().is_ok());
    }
}
