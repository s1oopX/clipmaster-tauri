use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{CleanupFileTarget, CleanupPlan, ClipboardItem};

use super::{
    clipboard_item_from_row, refresh_session_item_count, Database, CLIPBOARD_ITEM_COLUMNS,
};

impl Database {
    /// 获取自定义清理候选记录
    pub fn get_cleanup_candidates(
        &self,
        max_items: i32,
        keep_days: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let keep_threshold =
            Utc::now().timestamp_millis() - (keep_days as i64 * 24 * 60 * 60 * 1000);

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE is_pinned = 0
               AND is_favorite = 0
               AND (
                    timestamp < ?2
                    OR id IN (
                        SELECT id FROM (
                            SELECT id
                            FROM clipboard_items
                            WHERE is_pinned = 0 AND is_favorite = 0
                            ORDER BY timestamp DESC
                            LIMIT -1 OFFSET ?1
                        )
                    )
               )
             ORDER BY timestamp ASC",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query_map(
            params![max_items.max(0), keep_threshold],
            clipboard_item_from_row,
        )?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// 删除多条记录
    pub fn delete_items(&self, item_ids: &[String]) -> Result<()> {
        if item_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut affected_session_ids = Vec::new();

        for item_id in item_ids {
            if let Some(session_id) = tx
                .query_row(
                    "SELECT session_id FROM clipboard_items WHERE id = ?1",
                    params![item_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                if !affected_session_ids.contains(&session_id) {
                    affected_session_ids.push(session_id);
                }
            }

            tx.execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![item_id],
            )?;
        }

        for session_id in affected_session_ids {
            refresh_session_item_count(&tx, &session_id)?;
        }

        tx.commit()?;
        Ok(())
    }

    /// 清空全部剪贴板历史，保留当前活动会话并重置计数。
    pub fn clear_all_history(&self) -> Result<(CleanupPlan, Vec<CleanupFileTarget>)> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let plan = query_cleanup_plan(
            &tx,
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN type IN ('text', 'link') THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN type = 'image' THEN 1 ELSE 0 END), 0),
                    MIN(timestamp),
                    MAX(timestamp)
             FROM clipboard_items",
            [],
        )?;
        let file_targets = query_cleanup_file_targets(
            &tx,
            "SELECT id, image_path, thumbnail_path
             FROM clipboard_items
             WHERE type = 'image'
               AND (image_path IS NOT NULL OR thumbnail_path IS NOT NULL)",
            [],
        )?;

        tx.execute("DELETE FROM clipboard_items", [])?;
        tx.execute("DELETE FROM sessions WHERE is_active = 0", [])?;
        tx.execute("UPDATE sessions SET item_count = 0 WHERE is_active = 1", [])?;
        tx.commit()?;

        Ok((plan, file_targets))
    }

    /// 预览自定义清理结果
    pub fn cleanup_plan(&self, max_items: i32, keep_days: i32) -> Result<CleanupPlan> {
        let candidates = self.get_cleanup_candidates(max_items, keep_days)?;
        Ok(CleanupPlan::from_items(candidates))
    }
}

fn query_cleanup_plan<P>(conn: &Connection, sql: &str, params: P) -> Result<CleanupPlan>
where
    P: rusqlite::Params,
{
    conn.query_row(sql, params, |row| {
        Ok(CleanupPlan::from_counts(
            count_to_i32(row.get::<_, i64>(0)?),
            count_to_i32(row.get::<_, i64>(1)?),
            count_to_i32(row.get::<_, i64>(2)?),
            row.get(3)?,
            row.get(4)?,
        ))
    })
    .map_err(Into::into)
}

pub(super) fn query_cleanup_file_targets<P>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<CleanupFileTarget>>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare(sql)?;
    let targets = stmt
        .query_map(params, |row| {
            Ok(CleanupFileTarget {
                id: row.get(0)?,
                image_path: row.get(1)?,
                thumbnail_path: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(targets)
}

fn count_to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
