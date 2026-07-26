use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::link::{is_safe_web_url, link_content_hash, normalize_web_url};
use crate::models::{ClipboardDay, ClipboardItem, ClipboardType, CreateClipboardItem};

use super::utils::{date_key_from_timestamp, preview_from_content};
use super::{
    clipboard_item_from_row, fts_phrase_query, like_literal_pattern, refresh_duplicate_for_date,
    refresh_session_item_count, session_exists, Database, CLIPBOARD_ITEM_COLUMNS,
};

impl Database {
    pub fn rebuild_date_keys(&self, time_zone: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let items = {
            let mut stmt = tx.prepare("SELECT id, timestamp FROM clipboard_items")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        for (id, timestamp) in items {
            let date_key = date_key_from_timestamp(timestamp, time_zone);
            tx.execute(
                "UPDATE clipboard_items SET date_key = ?1 WHERE id = ?2",
                params![date_key, id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// 插入剪贴板记录；同一设置时区自然日内的相同内容只刷新记录时间。
    pub fn insert_item(&self, item: CreateClipboardItem, time_zone: &str) -> Result<ClipboardItem> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let session_id = item.session_id.clone();

        if !session_exists(&tx, &session_id)? {
            return Err(anyhow::anyhow!("会话不存在"));
        }

        let id = nanoid::nanoid!();
        let timestamp = Utc::now().timestamp_millis();
        let date_key = date_key_from_timestamp(timestamp, time_zone);

        if let Some(existing) =
            refresh_duplicate_for_date(&tx, &item.content_hash, &date_key, timestamp)?
        {
            tx.commit()?;
            return Ok(existing);
        }

        let preview = item.content.as_ref().map(|c| preview_from_content(c));

        tx.execute(
            "INSERT INTO clipboard_items (
                id, type, content, image_path, thumbnail_path, preview, timestamp, date_key,
                source_app, content_hash, session_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                item.type_.as_str(),
                item.content,
                item.image_path,
                item.thumbnail_path,
                preview,
                timestamp,
                date_key,
                item.source_app,
                item.content_hash,
                item.session_id,
            ],
        )?;
        refresh_session_item_count(&tx, &session_id)?;
        tx.commit()?;

        Ok(ClipboardItem {
            id: id.clone(),
            type_: item.type_,
            content: item.content,
            image_path: item.image_path,
            thumbnail_path: item.thumbnail_path,
            preview,
            timestamp,
            date_key,
            source_app: item.source_app,
            is_favorite: false,
            is_pinned: false,
            annotation: None,
            content_hash: item.content_hash,
            session_id: item.session_id,
        })
    }

    pub fn get_items_filtered(
        &self,
        limit: i32,
        offset: i32,
        item_type: Option<&str>,
        favorite_only: bool,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE (?3 IS NULL OR type = ?3)
               AND (?4 = 0 OR is_favorite = 1)
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?1 OFFSET ?2",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map(
                params![limit, offset, item_type, favorite_only as i32],
                clipboard_item_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 按会话获取记录
    pub fn get_items_by_session(
        &self,
        session_id: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE session_id = ?1
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?2 OFFSET ?3",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map(params![session_id, limit, offset], clipboard_item_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 如果设置时区当天已有相同内容，刷新该记录时间并返回它。
    pub fn refresh_duplicate_for_time_zone(
        &self,
        content_hash: &str,
        time_zone: &str,
    ) -> Result<Option<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let timestamp = Utc::now().timestamp_millis();
        let date_key = date_key_from_timestamp(timestamp, time_zone);

        refresh_duplicate_for_date(&conn, content_hash, &date_key, timestamp)
    }

    /// 获取单条剪贴板记录
    pub fn get_item(&self, item_id: &str) -> Result<Option<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE id = ?1",
            CLIPBOARD_ITEM_COLUMNS
        );
        let result = conn.query_row(&sql, params![item_id], clipboard_item_from_row);

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 删除记录
    pub fn delete_item(&self, item_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let session_id: String = tx
            .query_row(
                "SELECT session_id FROM clipboard_items WHERE id = ?1",
                params![item_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("记录不存在"))?;

        tx.execute(
            "DELETE FROM clipboard_items WHERE id = ?1",
            params![item_id],
        )?;
        refresh_session_item_count(&tx, &session_id)?;
        tx.commit()?;

        Ok(())
    }

    /// 切换收藏状态
    pub fn toggle_favorite(&self, item_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let is_favorite: i32 = conn
            .query_row(
                "SELECT is_favorite FROM clipboard_items WHERE id = ?1",
                params![item_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("记录不存在"))?;

        let new_state = if is_favorite == 1 { 0 } else { 1 };

        conn.execute(
            "UPDATE clipboard_items SET is_favorite = ?1 WHERE id = ?2",
            params![new_state, item_id],
        )?;

        Ok(new_state == 1)
    }

    /// 切换置顶状态
    pub fn toggle_pinned(&self, item_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let is_pinned: i32 = conn
            .query_row(
                "SELECT is_pinned FROM clipboard_items WHERE id = ?1",
                params![item_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("记录不存在"))?;

        let new_state = if is_pinned == 1 { 0 } else { 1 };

        conn.execute(
            "UPDATE clipboard_items SET is_pinned = ?1 WHERE id = ?2",
            params![new_state, item_id],
        )?;

        Ok(new_state == 1)
    }

    /// 更新记录内容
    pub fn update_item_content(&self, item_id: &str, new_content: &str) -> Result<ClipboardItem> {
        let conn = self.conn.lock().unwrap();
        let (item_type, date_key): (String, String) = conn
            .query_row(
                "SELECT type, date_key FROM clipboard_items WHERE id = ?1",
                params![item_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("记录不存在"))?;

        if item_type != ClipboardType::Text.as_str() {
            return Err(anyhow::anyhow!("只能编辑文本记录"));
        }

        if new_content.trim().is_empty() {
            return Err(anyhow::anyhow!("原文不能为空"));
        }

        let normalized_link = normalize_web_url(new_content);
        let content = if let Some(url) = normalized_link {
            url
        } else {
            new_content.to_string()
        };
        let next_type = if is_safe_web_url(&content) {
            ClipboardType::Link
        } else {
            ClipboardType::Text
        };
        let preview = preview_from_content(&content);
        let content_hash = if matches!(next_type, ClipboardType::Link) {
            link_content_hash(&content)
        } else {
            format!("{:x}", md5::compute(content.as_bytes()))
        };
        let duplicate_id = conn
            .query_row(
                "SELECT id FROM clipboard_items
                 WHERE content_hash = ?1 AND date_key = ?2 AND id != ?3
                 LIMIT 1",
                params![content_hash, date_key, item_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        if duplicate_id.is_some() {
            return Err(anyhow::anyhow!("当日已存在相同内容"));
        }

        conn.execute(
            "UPDATE clipboard_items
             SET type = ?1, content = ?2, preview = ?3, content_hash = ?4
             WHERE id = ?5",
            params![next_type.as_str(), content, preview, content_hash, item_id],
        )?;

        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE id = ?1",
            CLIPBOARD_ITEM_COLUMNS
        );
        conn.query_row(&sql, params![item_id], clipboard_item_from_row)
            .map_err(Into::into)
    }

    /// 更新记录标注，不修改原始内容和预览
    pub fn update_item_annotation(&self, item_id: &str, annotation: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let updated = if annotation.is_some() {
            conn.execute(
                "UPDATE clipboard_items SET annotation = ?1, is_favorite = 1 WHERE id = ?2",
                params![annotation, item_id],
            )?
        } else {
            conn.execute(
                "UPDATE clipboard_items SET annotation = ?1 WHERE id = ?2",
                params![annotation, item_id],
            )?
        };

        if updated == 0 {
            return Err(anyhow::anyhow!("记录不存在"));
        }

        Ok(())
    }

    /// 获取可用日期列表
    pub fn get_available_days(&self, limit: i32) -> Result<Vec<ClipboardDay>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT date_key, COUNT(*) AS item_count, MIN(timestamp), MAX(timestamp)
             FROM clipboard_items
             WHERE date_key IS NOT NULL AND date_key != ''
             GROUP BY date_key
             ORDER BY date_key DESC
             LIMIT ?1",
        )?;

        let days = stmt
            .query_map(params![limit], |row| {
                Ok(ClipboardDay {
                    date_key: row.get(0)?,
                    item_count: row.get(1)?,
                    start_time: row.get(2)?,
                    end_time: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(days)
    }

    pub fn get_items_by_day_filtered(
        &self,
        date_key: &str,
        limit: i32,
        offset: i32,
        item_type: Option<&str>,
        favorite_only: bool,
    ) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!(
            "SELECT {}
             FROM clipboard_items
             WHERE date_key = ?1
               AND (?4 IS NULL OR type = ?4)
               AND (?5 = 0 OR is_favorite = 1)
             ORDER BY is_pinned DESC, timestamp DESC
             LIMIT ?2 OFFSET ?3",
            CLIPBOARD_ITEM_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map(
                params![date_key, limit, offset, item_type, favorite_only as i32],
                clipboard_item_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// 搜索记录
    #[allow(clippy::too_many_arguments)]
    pub fn search_items(
        &self,
        query: &str,
        session_id: Option<&str>,
        date_key: &str,
        limit: i32,
        offset: i32,
        item_type: Option<&str>,
        favorite_only: bool,
    ) -> Result<Vec<ClipboardItem>> {
        let Some(like_pattern) = like_literal_pattern(query) else {
            return Ok(Vec::new());
        };
        // ≥3 字符走 trigram FTS5 索引；更短的查询回退 LIKE（语义一致：大小写不敏感的子串匹配）
        let (fts_phrase, use_fts) = match fts_phrase_query(query) {
            Some(phrase) => (phrase, true),
            None => (String::new(), false),
        };
        let search_param = if use_fts { &fts_phrase } else { &like_pattern };

        let conn = self.conn.lock().unwrap();

        // 根据是否有 session_id 分别执行不同的查询
        let items = if let Some(sid) = session_id {
            let match_clause = if use_fts {
                "rowid IN (SELECT rowid FROM clipboard_items_fts WHERE clipboard_items_fts MATCH ?3)"
            } else {
                "(
                       content LIKE ?3 ESCAPE '\\'
                       OR preview LIKE ?3 ESCAPE '\\'
                       OR annotation LIKE ?3 ESCAPE '\\'
                   )"
            };
            let sql = format!(
                "SELECT {}
                 FROM clipboard_items
                 WHERE date_key = ?1
                   AND session_id = ?2
                   AND (?6 IS NULL OR type = ?6)
                   AND (?7 = 0 OR is_favorite = 1)
                   AND {}
                 ORDER BY timestamp DESC
                 LIMIT ?4 OFFSET ?5",
                CLIPBOARD_ITEM_COLUMNS, match_clause
            );
            let mut stmt = conn.prepare(&sql)?;

            let rows = stmt.query_map(
                params![
                    date_key,
                    sid,
                    search_param,
                    limit,
                    offset,
                    item_type,
                    favorite_only as i32
                ],
                clipboard_item_from_row,
            )?;

            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            let match_clause = if use_fts {
                "rowid IN (SELECT rowid FROM clipboard_items_fts WHERE clipboard_items_fts MATCH ?2)"
            } else {
                "(
                       content LIKE ?2 ESCAPE '\\'
                       OR preview LIKE ?2 ESCAPE '\\'
                       OR annotation LIKE ?2 ESCAPE '\\'
                   )"
            };
            let sql = format!(
                "SELECT {}
                 FROM clipboard_items
                 WHERE date_key = ?1
                   AND (?5 IS NULL OR type = ?5)
                   AND (?6 = 0 OR is_favorite = 1)
                   AND {}
                 ORDER BY timestamp DESC
                 LIMIT ?3 OFFSET ?4",
                CLIPBOARD_ITEM_COLUMNS, match_clause
            );
            let mut stmt = conn.prepare(&sql)?;

            let rows = stmt.query_map(
                params![
                    date_key,
                    search_param,
                    limit,
                    offset,
                    item_type,
                    favorite_only as i32
                ],
                clipboard_item_from_row,
            )?;

            rows.collect::<Result<Vec<_>, _>>()?
        };

        Ok(items)
    }
}
