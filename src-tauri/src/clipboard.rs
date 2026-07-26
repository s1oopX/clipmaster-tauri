use anyhow::Result;
use arboard::Clipboard;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

use crate::app_data;
use crate::database::{date_key_now, Database};
use crate::link::{is_safe_web_url, link_content_hash, normalize_web_url};
use crate::models::{ClipboardType, CreateClipboardItem};
use crate::session::SessionManager;
use crate::settings::SettingsStore;

pub struct ClipboardService {
    last_hash: Arc<Mutex<String>>,
    last_sequence: Arc<Mutex<Option<u32>>>,
}

#[derive(Clone, Default)]
pub struct ClipboardWriteState {
    pending_hashes: Arc<Mutex<VecDeque<PendingClipboardWrite>>>,
}

struct PendingClipboardWrite {
    hash: String,
    created_at: Instant,
}

impl ClipboardWriteState {
    const MAX_PENDING_WRITES: usize = 16;
    const PENDING_WRITE_TTL: Duration = Duration::from_secs(5);

    pub fn suppress_next_hash(&self, hash: String) {
        let mut pending = self.pending_hashes.lock();
        Self::prune_expired(&mut pending);
        pending.push_back(PendingClipboardWrite {
            hash,
            created_at: Instant::now(),
        });

        while pending.len() > Self::MAX_PENDING_WRITES {
            pending.pop_front();
        }
    }

    pub fn consume_pending_hash(&self, hash: &str) -> bool {
        let mut pending = self.pending_hashes.lock();
        Self::prune_expired(&mut pending);
        match pending
            .iter()
            .position(|pending_write| pending_write.hash == hash)
        {
            Some(position) => {
                // 命中项之前排队的写入已被覆盖、永远不会被轮询观察到，
                // 一并清除，避免它们滞留后误吞用户后续的真实复制。
                pending.drain(..=position);
                true
            }
            None => {
                // 观察到的内容不是任何待消费的自写回，说明剪贴板已被外部
                // 内容接管：此前排队的自写回全部过时，立即清空。
                pending.clear();
                false
            }
        }
    }

    pub fn forget_pending_hash(&self, hash: &str) {
        let mut pending = self.pending_hashes.lock();
        Self::prune_expired(&mut pending);
        if let Some(position) = pending
            .iter()
            .position(|pending_write| pending_write.hash == hash)
        {
            pending.remove(position);
        }
    }

    fn prune_expired(pending: &mut VecDeque<PendingClipboardWrite>) {
        let now = Instant::now();
        pending.retain(|pending_write| {
            now.duration_since(pending_write.created_at) <= Self::PENDING_WRITE_TTL
        });
    }

    #[cfg(test)]
    fn suppress_expired_hash_for_test(&self, hash: String) {
        let mut pending = self.pending_hashes.lock();
        pending.push_back(PendingClipboardWrite {
            hash,
            created_at: Instant::now() - Self::PENDING_WRITE_TTL - Duration::from_millis(1),
        });
    }
}

impl ClipboardService {
    pub fn new() -> Self {
        Self {
            last_hash: Arc::new(Mutex::new(String::new())),
            last_sequence: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动剪贴板监听服务（带 panic 监督：轮询任务异常结束时自动重启）
    pub fn start(&self, app_handle: AppHandle) {
        let last_hash = Arc::clone(&self.last_hash);
        let last_sequence = Arc::clone(&self.last_sequence);

        tauri::async_runtime::spawn(async move {
            loop {
                let handle = tauri::async_runtime::spawn(Self::poll_loop(
                    app_handle.clone(),
                    Arc::clone(&last_hash),
                    Arc::clone(&last_sequence),
                ));

                match handle.await {
                    Ok(()) => break,
                    Err(error) => {
                        eprintln!("剪贴板监听任务异常结束，2 秒后自动重启: {}", error);
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        });
    }

    async fn poll_loop(
        app_handle: AppHandle,
        last_hash: Arc<Mutex<String>>,
        last_sequence: Arc<Mutex<Option<u32>>>,
    ) {
        let mut clipboard = loop {
            match Clipboard::new() {
                Ok(clipboard) => break clipboard,
                Err(error) => {
                    eprintln!("初始化剪贴板失败，将在 500ms 后重试: {}", error);
                    sleep(Duration::from_millis(500)).await;
                }
            }
        };

        loop {
            let settings = app_handle.state::<SettingsStore>();
            if !settings.get().clipboard_monitor_enabled {
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            let clipboard_sequence = clipboard_sequence_number();

            if Self::should_skip_sequence(&last_sequence, clipboard_sequence) {
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            // 尊重密码管理器等来源的隐私标记：跳过且不读取内容
            if clipboard_marked_private() {
                if let Some(sequence) = clipboard_sequence {
                    *last_sequence.lock() = Some(sequence);
                }
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            // 尝试读取剪贴板内容
            if let Ok(content) = Self::get_clipboard_content(&mut clipboard) {
                let hash = Self::calculate_hash(&content);

                if Self::should_skip_self_write(&app_handle, &hash) {
                    Self::mark_clipboard_item_saved(
                        &last_hash,
                        &last_sequence,
                        &hash,
                        clipboard_sequence,
                    );
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }

                let hash_changed = {
                    let last = last_hash.lock();
                    hash.as_str() != last.as_str()
                };
                let should_save = clipboard_sequence.is_some() || hash_changed;

                if should_save {
                    // 保存到数据库
                    match Self::save_clipboard_item(&app_handle, content, hash.clone()).await {
                        Ok(()) => {
                            Self::mark_clipboard_item_saved(
                                &last_hash,
                                &last_sequence,
                                &hash,
                                clipboard_sequence,
                            );
                        }
                        Err(e) => {
                            eprintln!("Failed to save clipboard item: {}", e);
                        }
                    }
                }
            }

            // 每 500ms 检查一次
            sleep(Duration::from_millis(500)).await;
        }
    }

    fn should_skip_sequence(
        last_sequence: &Arc<Mutex<Option<u32>>>,
        clipboard_sequence: Option<u32>,
    ) -> bool {
        clipboard_sequence
            .map(|sequence| *last_sequence.lock() == Some(sequence))
            .unwrap_or(false)
    }

    fn mark_clipboard_item_saved(
        last_hash: &Arc<Mutex<String>>,
        last_sequence: &Arc<Mutex<Option<u32>>>,
        hash: &str,
        clipboard_sequence: Option<u32>,
    ) {
        *last_hash.lock() = hash.to_string();
        if let Some(sequence) = clipboard_sequence {
            *last_sequence.lock() = Some(sequence);
        }
    }

    fn should_skip_self_write(app_handle: &AppHandle, hash: &str) -> bool {
        app_handle
            .try_state::<ClipboardWriteState>()
            .map(|state| state.consume_pending_hash(hash))
            .unwrap_or(false)
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
            ClipboardContent::Text(text) => calculate_text_clipboard_hash(text),
            ClipboardContent::Image(img) => {
                calculate_image_clipboard_hash(img.width, img.height, &img.bytes)
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
        let time_zone = app_handle.state::<SettingsStore>().get().time_zone;

        if let Some(saved_item) = db.refresh_duplicate_for_time_zone(&content_hash, &time_zone)? {
            app_handle.emit("clipboard:new-item", &saved_item)?;
            return Ok(());
        }

        // 获取当前会话ID
        let session_id = session_mgr
            .get_current_session_id()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?;

        // 创建记录
        let item = match content {
            ClipboardContent::Text(text) => {
                let normalized_link = normalize_web_url(&text);
                CreateClipboardItem {
                    type_: if normalized_link.is_some() {
                        ClipboardType::Link
                    } else {
                        ClipboardType::Text
                    },
                    content: Some(normalized_link.unwrap_or(text)),
                    image_path: None,
                    thumbnail_path: None,
                    source_app: None,
                    content_hash,
                    session_id,
                }
            }
            ClipboardContent::Image(img) => {
                // 保存图片到文件系统
                let (image_path, thumbnail_path) =
                    Self::save_image(app_handle, &img, &content_hash, &time_zone)?;

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

        // 插入数据库；图片记录插入失败时回收刚落盘的文件，避免孤儿
        let image_files = (item.image_path.clone(), item.thumbnail_path.clone());
        let saved_item = match db.insert_item(item, &time_zone) {
            Ok(saved_item) => saved_item,
            Err(error) => {
                if let Ok(app_data_dir) = app_data::resolve_app_data_dir(app_handle) {
                    for relative_path in [image_files.0, image_files.1].into_iter().flatten() {
                        let _ = fs::remove_file(app_data_dir.join(relative_path));
                    }
                }
                return Err(error);
            }
        };

        // 通知前端
        app_handle.emit("clipboard:new-item", &saved_item)?;

        Ok(())
    }

    /// 保存图片到文件系统
    fn save_image(
        app_handle: &AppHandle,
        img: &arboard::ImageData,
        content_hash: &str,
        time_zone: &str,
    ) -> Result<(String, String)> {
        // 获取应用数据目录
        let app_data_dir = app_data::resolve_app_data_dir(app_handle)?;

        // 创建按日期分组的图片目录
        let date_key = date_key_now(time_zone);
        let images_dir = app_data_dir.join("images").join(&date_key);

        // 确保目录存在
        fs::create_dir_all(&images_dir)?;

        // 生成文件名: hash前8位_时间戳.png
        let timestamp = chrono::Utc::now().timestamp_millis();
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

        // 返回相对路径: images/2026-06-06/hash_timestamp.png
        let relative_path = format!("images/{}/{}", date_key, filename);
        let relative_thumb_path = format!("images/{}/{}", date_key, thumb_filename);

        Ok((relative_path, relative_thumb_path))
    }
}

/// 剪贴板内容是否带有"请勿记录"的隐私标记。
///
/// 密码管理器（KeePass / 1Password / Bitwarden 等）复制密码时会设置
/// `ExcludeClipboardContentFromMonitorProcessing` 或值为 0 的
/// `CanIncludeInClipboardHistory` 剪贴板格式，剪贴板管理器应当遵守。
#[cfg(target_os = "windows")]
fn clipboard_marked_private() -> bool {
    use std::sync::OnceLock;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    fn register_format(name: &str) -> u32 {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { RegisterClipboardFormatW(wide.as_ptr()) }
    }

    static EXCLUDE_FORMAT: OnceLock<u32> = OnceLock::new();
    static CAN_INCLUDE_FORMAT: OnceLock<u32> = OnceLock::new();

    let exclude_format = *EXCLUDE_FORMAT
        .get_or_init(|| register_format("ExcludeClipboardContentFromMonitorProcessing"));
    let can_include_format =
        *CAN_INCLUDE_FORMAT.get_or_init(|| register_format("CanIncludeInClipboardHistory"));

    unsafe {
        if exclude_format != 0 && IsClipboardFormatAvailable(exclude_format) != 0 {
            return true;
        }

        if can_include_format != 0 && IsClipboardFormatAvailable(can_include_format) != 0 {
            // 格式存在时读取 DWORD 值：0 = 明确禁止入历史。
            // 读取失败按隐私内容处理——写入方已明确表达了偏好。
            if OpenClipboard(0) == 0 {
                return true;
            }
            let mut private = true;
            let handle = GetClipboardData(can_include_format);
            if handle != 0 {
                let pointer = GlobalLock(handle) as *const u32;
                if !pointer.is_null() {
                    private = *pointer == 0;
                    GlobalUnlock(handle);
                }
            }
            CloseClipboard();
            return private;
        }
    }

    false
}

#[cfg(not(target_os = "windows"))]
fn clipboard_marked_private() -> bool {
    false
}

pub fn calculate_text_clipboard_hash(text: &str) -> String {
    if is_safe_web_url(text) {
        link_content_hash(text)
    } else {
        format!("{:x}", md5::compute(text.as_bytes()))
    }
}

pub fn calculate_image_clipboard_hash(width: usize, height: usize, bytes: &[u8]) -> String {
    // 全量哈希：采样哈希会让小差异图片（光标、计时器数字）碰撞，
    // 命中去重后新图被静默丢弃。哈希只在剪贴板序列号变化时计算一次，
    // 全量 MD5 对 4K 截图也只有毫秒级开销。
    let mut context = md5::Context::new();
    context.consume(width.to_le_bytes());
    context.consume(height.to_le_bytes());
    context.consume(bytes);
    format!("{:x}", context.compute())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_only_safe_web_urls() {
        assert!(is_safe_web_url("https://example.com"));
        assert!(is_safe_web_url(
            " http://docs.example.com/path?q=1#install "
        ));
        assert!(!is_safe_web_url("https://localhost"));
    }

    #[test]
    fn pending_self_write_hash_is_consumed_once() {
        let state = ClipboardWriteState::default();

        state.suppress_next_hash("alpha".to_string());

        assert!(state.consume_pending_hash("alpha"));
        assert!(!state.consume_pending_hash("alpha"));
    }

    #[test]
    fn pending_self_write_hashes_are_bounded() {
        let state = ClipboardWriteState::default();

        for index in 0..(ClipboardWriteState::MAX_PENDING_WRITES + 2) {
            state.suppress_next_hash(format!("hash-{index}"));
        }

        // 只保留最近 16 条：最早两条被挤出，最早的幸存者是 hash-2
        assert!(state.consume_pending_hash("hash-2"));
        assert!(state.consume_pending_hash("hash-3"));
    }

    #[test]
    fn consuming_pending_hash_drops_older_queued_writes() {
        let state = ClipboardWriteState::default();

        state.suppress_next_hash("older".to_string());
        state.suppress_next_hash("newer".to_string());

        // 轮询先观察到较新的写入：更早排队的写入已被覆盖、不可能再被观察到
        assert!(state.consume_pending_hash("newer"));
        assert!(!state.consume_pending_hash("older"));
    }

    #[test]
    fn external_content_clears_stale_pending_hashes() {
        let state = ClipboardWriteState::default();

        state.suppress_next_hash("self-write".to_string());

        // 观察到外部内容后，滞留的自写回全部过时；
        // 随后用户复制相同内容不应再被误吞
        assert!(!state.consume_pending_hash("external"));
        assert!(!state.consume_pending_hash("self-write"));
    }

    #[test]
    fn pending_self_write_hash_expires() {
        let state = ClipboardWriteState::default();

        state.suppress_expired_hash_for_test("alpha".to_string());

        assert!(!state.consume_pending_hash("alpha"));
    }

    #[test]
    fn image_clipboard_hash_includes_dimensions_and_pixels() {
        let pixels = [0, 10, 20, 30, 40, 50, 60, 70];

        let hash = calculate_image_clipboard_hash(1, 2, &pixels);

        assert_eq!(hash, calculate_image_clipboard_hash(1, 2, &pixels));
        assert_ne!(hash, calculate_image_clipboard_hash(2, 1, &pixels));
        assert_ne!(hash, calculate_image_clipboard_hash(1, 2, &[0, 10, 21]));
    }
}
