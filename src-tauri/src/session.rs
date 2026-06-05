use parking_lot::Mutex;
use std::sync::Arc;

pub struct SessionManager {
    current_session_id: Arc<Mutex<Option<String>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            current_session_id: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动新会话
    pub fn start_new_session(&self) -> String {
        let session_id = format!("session_{}", nanoid::nanoid!());
        *self.current_session_id.lock() = Some(session_id.clone());
        session_id
    }

    /// 获取当前会话ID
    pub fn get_current_session_id(&self) -> Option<String> {
        self.current_session_id.lock().clone()
    }

    /// 结束当前会话
    pub fn end_current_session(&self) {
        *self.current_session_id.lock() = None;
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
