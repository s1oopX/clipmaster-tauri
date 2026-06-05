import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * 剪贴板 API
 */
export const clipboardApi = {
  /**
   * 获取剪贴板列表
   * @param {number} limit - 返回数量
   * @param {number} offset - 偏移量
   */
  async getItems(limit = 100, offset = 0) {
    return await invoke('get_clipboard_items', { limit, offset });
  },

  /**
   * 删除记录
   * @param {string} itemId
   */
  async deleteItem(itemId) {
    return await invoke('delete_item', { itemId });
  },

  /**
   * 切换收藏
   * @param {string} itemId
   * @returns {boolean} 新的收藏状态
   */
  async toggleFavorite(itemId) {
    return await invoke('toggle_favorite', { itemId });
  },

  /**
   * 切换置顶
   * @param {string} itemId
   * @returns {boolean} 新的置顶状态
   */
  async togglePinned(itemId) {
    return await invoke('toggle_pinned', { itemId });
  },

  /**
   * 监听新记录事件
   * @param {Function} callback
   * @returns {Promise<Function>} unlisten 函数
   */
  async onNewItem(callback) {
    return await listen('clipboard:new-item', (event) => {
      callback(event.payload);
    });
  },
};

/**
 * 会话 API
 */
export const sessionApi = {
  /**
   * 获取当前会话
   */
  async getCurrentSession() {
    return await invoke('get_current_session');
  },

  /**
   * 获取历史会话列表
   * @param {number} limit
   */
  async getSessions(limit = 50) {
    return await invoke('get_sessions', { limit });
  },

  /**
   * 按会话获取记录
   * @param {string} sessionId
   * @param {number} limit
   * @param {number} offset
   */
  async getItemsBySession(sessionId, limit = 100, offset = 0) {
    return await invoke('get_items_by_session', {
      sessionId,
      limit,
      offset,
    });
  },

  /**
   * 清空会话
   * @param {string} sessionId
   */
  async clearSession(sessionId) {
    return await invoke('clear_session', { sessionId });
  },
};

/**
 * 搜索 API
 */
export const searchApi = {
  /**
   * 搜索记录
   * @param {string} query - 搜索关键词
   * @param {string} sessionId - 可选：会话ID
   * @param {number} limit
   */
  async searchItems(query, sessionId = null, limit = 100) {
    return await invoke('search_items', {
      query,
      sessionId,
      limit,
    });
  },
};
