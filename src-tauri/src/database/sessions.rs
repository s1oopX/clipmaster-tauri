use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

use crate::models::{CleanupFileTarget, Session};

use super::{cleanup::query_cleanup_file_targets, Database};

impl Database {
    /// 创建会话
    pub fn create_session(&self, session_id: &str) -> Result<()> {
        let conn = self.lock_conn();
        let now = Utc::now().timestamp_millis();

        // 结束所有活跃会话
        conn.execute(
            "UPDATE sessions
             SET is_active = 0,
                 end_time = ?1,
                 item_count = (
                    SELECT COUNT(*)
                    FROM clipboard_items
                    WHERE clipboard_items.session_id = sessions.id
                 )
             WHERE is_active = 1",
            params![now],
        )?;

        // 创建新会话
        conn.execute(
            "INSERT INTO sessions (id, start_time, is_active) VALUES (?1, ?2, 1)",
            params![session_id, now],
        )?;

        Ok(())
    }

    /// 结束会话
    pub fn end_session(&self, session_id: &str) -> Result<()> {
        let conn = self.lock_conn();
        let now = Utc::now().timestamp_millis();

        // 统计记录数
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        // 更新会话
        conn.execute(
            "UPDATE sessions SET end_time = ?1, item_count = ?2, is_active = 0 WHERE id = ?3",
            params![now, count, session_id],
        )?;

        Ok(())
    }

    /// 获取当前会话
    pub fn get_current_session(&self) -> Result<Option<Session>> {
        let conn = self.lock_conn();

        let result = conn.query_row(
            "SELECT id, start_time, end_time, item_count, is_active
             FROM sessions WHERE is_active = 1 LIMIT 1",
            [],
            |row| {
                Ok(Session {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    item_count: row.get(3)?,
                    is_active: row.get::<_, i32>(4)? == 1,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 获取会话列表
    pub fn get_sessions(&self, limit: i32) -> Result<Vec<Session>> {
        let conn = self.lock_conn();

        let mut stmt = conn.prepare(
            "SELECT id, start_time, end_time, item_count, is_active
             FROM sessions
             ORDER BY start_time DESC
             LIMIT ?1",
        )?;

        let sessions = stmt
            .query_map(params![limit], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    item_count: row.get(3)?,
                    is_active: row.get::<_, i32>(4)? == 1,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// 清空会话
    pub fn clear_session(&self, session_id: &str) -> Result<Vec<CleanupFileTarget>> {
        let mut conn = self.lock_conn();
        let tx = conn.transaction()?;

        let session_exists: i32 = tx.query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        if session_exists == 0 {
            return Err(anyhow::anyhow!("会话不存在"));
        }

        let file_targets = query_cleanup_file_targets(
            &tx,
            "SELECT id, image_path, thumbnail_path
             FROM clipboard_items
             WHERE session_id = ?1
               AND type = 'image'
               AND (image_path IS NOT NULL OR thumbnail_path IS NOT NULL)",
            params![session_id],
        )?;

        tx.execute(
            "DELETE FROM clipboard_items WHERE session_id = ?1",
            params![session_id],
        )?;

        tx.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;

        tx.commit()?;

        Ok(file_targets)
    }
}
