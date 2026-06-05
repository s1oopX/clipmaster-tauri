// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod clipboard;
mod database;
mod models;
mod session;

use clipboard::ClipboardService;
use database::Database;
use session::SessionManager;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 获取应用数据目录
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // 初始化数据库
            let db = Database::new(app_data_dir).expect("Failed to initialize database");

            // 初始化会话管理器
            let session_mgr = SessionManager::new();
            let session_id = session_mgr.start_new_session();

            // 在数据库中创建会话
            db.create_session(&session_id)
                .expect("Failed to create session");

            println!("Session started: {}", session_id);

            // 将服务注入到应用状态
            app.manage(db);
            app.manage(session_mgr);

            // 启动剪贴板监听服务
            let clipboard_service = ClipboardService::new();
            clipboard_service.start(app.handle().clone());

            println!("ClipMaster started successfully!");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // 窗口关闭时结束会话
                let app_handle = window.app_handle();
                let session_mgr = app_handle.state::<SessionManager>();
                let db = app_handle.state::<Database>();

                if let Some(session_id) = session_mgr.get_current_session_id() {
                    if let Err(e) = db.end_session(&session_id) {
                        eprintln!("Failed to end session: {}", e);
                    } else {
                        println!("Session ended: {}", session_id);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_clipboard_items,
            commands::get_items_by_session,
            commands::delete_item,
            commands::toggle_favorite,
            commands::toggle_pinned,
            commands::get_current_session,
            commands::get_sessions,
            commands::clear_session,
            commands::search_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
