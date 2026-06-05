use anyhow::Result;
use arboard::Clipboard;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::database::Database;
use crate::models::{ClipboardType, CreateClipboardItem};
use crate::session::SessionManager;

pub struct ClipboardService {
    last_hash: Arc<Mutex<String>>,
}

impl ClipboardService {
    pub fn new() -> Self {
        Self {
            last_hash: Arc::new(Mutex::new(String::new())),
        }
    }

    /// 启动剪贴板监听服务
    pub fn start(&self, app_handle: AppHandle) {
        let last_hash = Arc::clone(&self.last_hash);

        tauri::async_runtime::spawn(async move {
            let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");

            loop {
                // 尝试读取剪贴板内容
                if let Ok(content) = Self::get_clipboard_content(&mut clipboard) {
                    let hash = Self::calculate_hash(&content);

                    // 检查是否重复
                    let should_save = {
                        let mut last = last_hash.lock();
                        if hash != *last {
                            *last = hash.clone();
                            true
                        } else {
                            false
                        }
                    }; // 锁在这里自动释放

                    if should_save {
                        // 保存到数据库
                        if let Err(e) = Self::save_clipboard_item(&app_handle, content, hash).await
                        {
                            eprintln!("Failed to save clipboard item: {}", e);
                        }
                    }
                }

                // 每 500ms 检查一次
                sleep(Duration::from_millis(500)).await;
            }
        });
    }

    /// 获取剪贴板内容
    fn get_clipboard_content(clipboard: &mut Clipboard) -> Result<ClipboardContent> {
        // 优先检测图片
        if let Ok(image) = clipboard.get_image() {
            return Ok(ClipboardContent::Image(image));
        }

        // 检测文本
        if let Ok(text) = clipboard.get_text() {
            if !text.trim().is_empty() {
                return Ok(ClipboardContent::Text(text));
            }
        }

        anyhow::bail!("No clipboard content")
    }

    /// 计算内容哈希
    fn calculate_hash(content: &ClipboardContent) -> String {
        match content {
            ClipboardContent::Text(text) => {
                format!("{:x}", md5::compute(text.as_bytes()))
            }
            ClipboardContent::Image(img) => {
                // 使用图片的宽高和部分像素数据计算哈希
                let data = format!("{}x{}", img.width, img.height);
                format!("{:x}", md5::compute(data.as_bytes()))
            }
        }
    }

    /// 保存剪贴板记录
    async fn save_clipboard_item(
        app_handle: &AppHandle,
        content: ClipboardContent,
        content_hash: String,
    ) -> Result<()> {
        let db = app_handle.state::<Database>();
        let session_mgr = app_handle.state::<SessionManager>();

        // 检查是否重复（5分钟内）
        let time_window_ms = 5 * 60 * 1000; // 5分钟
        if db.has_duplicate(&content_hash, time_window_ms)? {
            return Ok(()); // 跳过重复内容
        }

        // 获取当前会话ID
        let session_id = session_mgr
            .get_current_session_id()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?;

        // 创建记录
        let item = match content {
            ClipboardContent::Text(text) => CreateClipboardItem {
                type_: ClipboardType::Text,
                content: Some(text),
                image_path: None,
                source_app: None,
                content_hash,
                session_id,
            },
            ClipboardContent::Image(_img) => {
                // TODO: 保存图片到文件系统
                CreateClipboardItem {
                    type_: ClipboardType::Image,
                    content: None,
                    image_path: Some("placeholder.png".to_string()),
                    source_app: None,
                    content_hash,
                    session_id,
                }
            }
        };

        // 插入数据库
        let saved_item = db.insert_item(item)?;

        // 通知前端
        app_handle.emit("clipboard:new-item", &saved_item)?;

        Ok(())
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}

/// 剪贴板内容枚举
enum ClipboardContent {
    Text(String),
    Image(arboard::ImageData<'static>),
}
