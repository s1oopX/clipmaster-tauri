use serde::{Deserialize, Serialize};

/// 剪贴板记录类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardType {
    Text,
    Link,
    Image,
    File,
}

impl ClipboardType {
    pub fn as_str(&self) -> &str {
        match self {
            ClipboardType::Text => "text",
            ClipboardType::Link => "link",
            ClipboardType::Image => "image",
            ClipboardType::File => "file",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(ClipboardType::Text),
            "link" => Some(ClipboardType::Link),
            "image" => Some(ClipboardType::Image),
            "file" => Some(ClipboardType::File),
            _ => None,
        }
    }
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub item_count: i32,
    pub is_active: bool,
}

/// 每日剪贴板记录聚合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardDay {
    pub date_key: String,
    pub item_count: i32,
    pub start_time: i64,
    pub end_time: i64,
}

/// 剪贴板记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: ClipboardType,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub preview: Option<String>,
    pub timestamp: i64,
    pub date_key: String,
    pub source_app: Option<String>,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub annotation: Option<String>,
    pub content_hash: String,
    pub session_id: String,
}

/// 创建剪贴板记录的参数
#[derive(Debug, Clone)]
pub struct CreateClipboardItem {
    pub type_: ClipboardType,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub source_app: Option<String>,
    pub content_hash: String,
    pub session_id: String,
}

/// 自定义清理预览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPlan {
    pub item_count: i32,
    pub text_count: i32,
    pub image_count: i32,
    pub oldest_timestamp: Option<i64>,
    pub newest_timestamp: Option<i64>,
}

impl CleanupPlan {
    pub fn from_counts(
        item_count: i32,
        text_count: i32,
        image_count: i32,
        oldest_timestamp: Option<i64>,
        newest_timestamp: Option<i64>,
    ) -> Self {
        Self {
            item_count,
            text_count,
            image_count,
            oldest_timestamp,
            newest_timestamp,
        }
    }

    pub fn from_items(items: Vec<ClipboardItem>) -> Self {
        let item_count = items.len() as i32;
        let text_count = items
            .iter()
            .filter(|item| matches!(item.type_, ClipboardType::Text | ClipboardType::Link))
            .count() as i32;
        let image_count = items
            .iter()
            .filter(|item| matches!(item.type_, ClipboardType::Image))
            .count() as i32;
        let oldest_timestamp = items.iter().map(|item| item.timestamp).min();
        let newest_timestamp = items.iter().map(|item| item.timestamp).max();

        Self {
            item_count,
            text_count,
            image_count,
            oldest_timestamp,
            newest_timestamp,
        }
    }
}

/// 删除历史后需要同步清理的图片文件。
#[derive(Debug, Clone)]
pub struct CleanupFileTarget {
    pub id: String,
    pub image_path: Option<String>,
    pub thumbnail_path: Option<String>,
}

impl CleanupFileTarget {
    pub fn from_item(item: &ClipboardItem) -> Option<Self> {
        if !matches!(item.type_, ClipboardType::Image) {
            return None;
        }

        Some(Self {
            id: item.id.clone(),
            image_path: item.image_path.clone(),
            thumbnail_path: item.thumbnail_path.clone(),
        })
    }
}
