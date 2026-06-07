use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

use crate::database::Database;
use crate::session::SessionManager;

const TRAY_MENU_SHOW: &str = "show";
const TRAY_MENU_QUIT: &str = "quit";

pub fn setup_tray<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, TRAY_MENU_SHOW, "显示 ClipMaster", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, TRAY_MENU_QUIT, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("ClipMaster")
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_MENU_SHOW => show_main_window(app),
            TRAY_MENU_QUIT => quit_app(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if is_primary_click(&event) {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

pub fn register_main_window_close_handler<R: Runtime>(window: &WebviewWindow<R>) {
    let window = window.clone();

    window.clone().on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            hide_main_webview_window_to_tray(&window);
        }
    });
}

pub fn hide_main_webview_window_to_tray<R: Runtime>(window: &WebviewWindow<R>) {
    if let Err(error) = window.hide() {
        eprintln!("Failed to hide main window to tray: {}", error);
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            eprintln!("Failed to show main window: {}", error);
            return;
        }

        if let Err(error) = window.set_focus() {
            eprintln!("Failed to focus main window: {}", error);
        }
    }
}

pub fn quit_app<R: Runtime>(app: &AppHandle<R>) {
    end_current_session(app);
    app.exit(0);
}

pub fn end_current_session<R: Runtime>(app: &AppHandle<R>) {
    let session_mgr = app.state::<SessionManager>();
    let db = app.state::<Database>();

    if let Some(session_id) = session_mgr.get_current_session_id() {
        if let Err(error) = db.end_session(&session_id) {
            eprintln!("Failed to end session: {}", error);
            return;
        }

        session_mgr.end_current_session();
        println!("Session ended: {}", session_id);
    }
}

fn is_primary_click(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
    use tauri::{PhysicalPosition, PhysicalSize, Position, Rect, Size};

    fn click_event(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: "main".into(),
            position: PhysicalPosition { x: 0.0, y: 0.0 },
            rect: Rect {
                position: Position::Physical(PhysicalPosition { x: 0, y: 0 }),
                size: Size::Physical(PhysicalSize {
                    width: 16,
                    height: 16,
                }),
            },
            button,
            button_state,
        }
    }

    #[test]
    fn primary_tray_click_is_left_button_release() {
        assert!(is_primary_click(&click_event(
            MouseButton::Left,
            MouseButtonState::Up
        )));

        assert!(!is_primary_click(&click_event(
            MouseButton::Left,
            MouseButtonState::Down
        )));
        assert!(!is_primary_click(&click_event(
            MouseButton::Right,
            MouseButtonState::Up
        )));
    }
}
