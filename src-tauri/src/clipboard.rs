use anyhow::Result;
use arboard::Clipboard;
use chrono::Local;
use parking_lot::Mutex;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::database::Database;
use crate::models::{ClipboardType, CreateClipboardItem};
use crate::session::SessionManager;
use crate::settings::SettingsStore;

pub struct ClipboardService {
    last_hash: Arc<Mutex<String>>,
    last_sequence: Arc<Mutex<Option<u32>>>,
}

impl ClipboardService {
    pub fn new() -> Self {
        Self {
            last_hash: Arc::new(Mutex::new(String::new())),
            last_sequence: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动剪贴板监听服务
    pub fn start(&self, app_handle: AppHandle) {
        let last_hash = Arc::clone(&self.last_hash);
        let last_sequence = Arc::clone(&self.last_sequence);

        tauri::async_runtime::spawn(async move {
            let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");

            loop {
                let settings = app_handle.state::<SettingsStore>();
                if !settings.get().clipboard_monitor_enabled {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }

                if let Some(sequence) = clipboard_sequence_number() {
                    let should_skip = {
                        let mut last = last_sequence.lock();
                        if *last == Some(sequence) {
                            true
                        } else {
                            *last = Some(sequence);
                            false
                        }
                    };

                    if should_skip {
                        sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                }

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
                // 使用图片的宽高和采样像素数据计算哈希
                let mut hash_data = Vec::new();

                // 添加宽高信息
                hash_data.extend_from_slice(&img.width.to_le_bytes());
                hash_data.extend_from_slice(&img.height.to_le_bytes());

                // 采样策略：取部分像素点（每隔100个像素取一个）
                // 避免处理整个图片数据，提升性能
                let sample_step = 100.min(img.bytes.len() / 100).max(1);
                for i in (0..img.bytes.len()).step_by(sample_step) {
                    hash_data.push(img.bytes[i]);
                }

                format!("{:x}", md5::compute(&hash_data))
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
            ClipboardContent::Image(img) => {
                // 保存图片到文件系统
                let (image_path, thumbnail_path) = Self::save_image(app_handle, &img, &content_hash)?;

                CreateClipboardItem {
                    type_: ClipboardType::Image,
                    content: None,
                    image_path: Some(image_path),
                    thumbnail_path: Some(thumbnail_path),
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

    /// 保存图片到文件系统
    fn save_image(
        app_handle: &AppHandle,
        img: &arboard::ImageData,
        content_hash: &str,
    ) -> Result<(String, String)> {
        // 获取应用数据目录
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;

        // 创建按月份分组的图片目录
        let year_month = Local::now().format("%Y-%m").to_string();
        let images_dir = app_data_dir.join("images").join(&year_month);

        // 确保目录存在
        fs::create_dir_all(&images_dir)?;

        // 生成文件名: hash前8位_时间戳.png
        let timestamp = chrono::Utc::now().timestamp();
        let filename = format!(
            "{}_{}.png",
            &content_hash[..8.min(content_hash.len())],
            timestamp
        );
        let file_path = images_dir.join(&filename);

        // 将 arboard::ImageData 转换为 image crate 的格式
        let image_buffer =
            image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.to_vec())
                .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

        // 保存原图为 PNG 格式
        image_buffer.save(&file_path)?;

        // 生成缩略图
        let thumb_filename = format!(
            "{}_{}_thumb.png",
            &content_hash[..8.min(content_hash.len())],
            timestamp
        );
        let thumb_path = images_dir.join(&thumb_filename);

        let thumb_image = image::imageops::resize(
            &image_buffer,
            200,
            200,
            image::imageops::FilterType::Lanczos3,
        );
        thumb_image.save(&thumb_path)?;

        // 返回相对路径: images/2026-06/hash_timestamp.png
        let relative_path = format!("images/{}/{}", year_month, filename);
        let relative_thumb_path = format!("images/{}/{}", year_month, thumb_filename);

        Ok((relative_path, relative_thumb_path))
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

#[cfg(target_os = "windows")]
fn clipboard_sequence_number() -> Option<u32> {
    #[link(name = "user32")]
    extern "system" {
        fn GetClipboardSequenceNumber() -> u32;
    }

    let sequence = unsafe { GetClipboardSequenceNumber() };

    if sequence == 0 {
        None
    } else {
        Some(sequence)
    }
}

#[cfg(not(target_os = "windows"))]
fn clipboard_sequence_number() -> Option<u32> {
    None
}
