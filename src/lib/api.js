import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';

// 确保 invoke 函数可用
if (typeof invoke !== 'function') {
  console.error('invoke function not available from @tauri-apps/api/core');
}

// 缓存应用数据目录路径
let appDataDir = null;

/**
 * 获取应用数据目录
 */
async function getAppDataDir() {
  if (!appDataDir) {
    appDataDir = await invoke('get_app_data_dir');
  }
  return appDataDir;
}

/**
 * 将相对路径转换为可访问的文件 URL
 * @param {string} relativePath - 相对路径，如 "images/2026-06/xxx.png"
 * @returns {Promise<string>} 可访问的文件 URL
 */
export async function convertImagePath(relativePath) {
  if (!relativePath) return null;

  try {
    const dataDir = await getAppDataDir();
    const normalizedRelPath = relativePath.replace(/[\\/]+/g, '\\');
    const fullPath = `${dataDir}\\${normalizedRelPath}`;

    return convertFileSrc(fullPath);
  } catch (error) {
    console.error('Error converting image path:', error);
    return null;
  }
}

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
   * 按日期获取剪贴板列表
   * @param {string} dateKey - 日期，如 "2026-06-06"
   * @param {number} limit - 返回数量
   * @param {number} offset - 偏移量
   */
  async getItemsByDay(dateKey, limit = 100, offset = 0) {
    return await invoke('get_items_by_day', { dateKey, limit, offset });
  },

  /**
   * 获取有记录的日期列表
   * @param {number} limit - 返回数量
   */
  async getAvailableDays(limit = 365) {
    return await invoke('get_available_days', { limit });
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
   * 复制文本到剪贴板
   * @param {string} text
   */
  async copyToClipboard(text) {
    return await invoke('copy_to_clipboard', { text });
  },

  /**
   * 复制图片到剪贴板
   * @param {string} imagePath - 应用数据目录下的相对图片路径
   */
  async copyImageToClipboard(imagePath) {
    return await invoke('copy_image_to_clipboard', { imagePath });
  },

  /**
   * 更新记录内容
   * @param {string} itemId
   * @param {string} newContent
   */
  async updateItemContent(itemId, newContent) {
    return await invoke('update_item_content', { itemId, newContent });
  },

  /**
   * 更新记录标注，不修改原始内容
   * @param {string} itemId
   * @param {string} annotation
   * @returns {Promise<string|null>} 保存后的标注，空字符串会被清除为 null
   */
  async updateItemAnnotation(itemId, annotation) {
    return await invoke('update_item_annotation', { itemId, annotation });
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
   * @param {string} dateKey - 限定日期，如 "2026-06-06"
   */
  async searchItems(query, sessionId = null, limit = 100, dateKey = null) {
    return await invoke('search_items', {
      query,
      sessionId,
      limit,
      dateKey,
    });
  },
};

/**
 * 工具 API
 */
export const toolApi = {
  /**
   * 开始区域截图
   */
  async startRegionScreenshot() {
    return await invoke('start_region_screenshot');
  },

  /**
   * 将图片钉到桌面
   * @param {string} imagePath - 应用数据目录下的相对图片路径
   */
  async pinImage(imagePath) {
    return await invoke('pin_image', { imagePath });
  },

  /**
   * 使用系统默认浏览器打开允许的外部链接
   * @param {string} url
   */
  async openExternalUrl(url) {
    return await invoke('open_external_url', { url });
  },
};

/**
 * 设置 API
 */
export const settingsApi = {
  /**
   * 获取应用设置
   */
  async getSettings() {
    return await invoke('get_settings');
  },

  /**
   * 保存应用设置
   * @param {object} settings
   */
  async saveSettings(settings) {
    return await invoke('save_settings', { settings });
  },

  /**
   * 检查开发端口是否可用
   * @param {number} port
   */
  async checkDevServerPort(port) {
    return await invoke('check_dev_server_port', { port });
  },

  /**
   * 重启应用
   */
  async restartApp() {
    return await invoke('restart_app');
  },

  /**
   * 预览自定义清理
   * @param {number} maxItems
   * @param {number} keepDays
   */
  async previewCustomCleanup(maxItems, keepDays) {
    return await invoke('preview_custom_cleanup', { maxItems, keepDays });
  },

  /**
   * 执行自定义清理
   * @param {number} maxItems
   * @param {number} keepDays
   */
  async runCustomCleanup(maxItems, keepDays) {
    return await invoke('run_custom_cleanup', { maxItems, keepDays });
  },
};
