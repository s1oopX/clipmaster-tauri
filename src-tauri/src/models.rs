use serde::{Deserialize, Serialize};

/// 剪贴板记录类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardType {
    Text,
    Image,
    File,
}

impl ClipboardType {
    pub fn as_str(&self) -> &str {
        match self {
            ClipboardType::Text => "text",
            ClipboardType::Image => "image",
            ClipboardType::File => "file",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(ClipboardType::Text),
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

/// 剪贴板记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: ClipboardType,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub preview: Option<String>,
    pub timestamp: i64,
    pub source_app: Option<String>,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub content_hash: String,
    pub session_id: String,
}

/// 创建剪贴板记录的参数
#[derive(Debug, Clone)]
pub struct CreateClipboardItem {
    pub type_: ClipboardType,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub content_hash: String,
    pub session_id: String,
}
