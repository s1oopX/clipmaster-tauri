import { defaultSettings } from './app-config.js';

export function formatDateKey(date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export function todayDateKey(timeZone) {
  try {
    const parts = new Intl.DateTimeFormat('en-CA', {
      timeZone: timeZone || defaultSettings.time_zone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    }).formatToParts(new Date());
    const values = Object.fromEntries(parts.map((part) => [part.type, part.value]));
    return `${values.year}-${values.month}-${values.day}`;
  } catch (_e) {
    return formatDateKey(new Date());
  }
}

export function itemMatchesSearchQuery(item, query) {
  if (!query) return true;

  const normalizedQuery = query.toLowerCase();
  return [item.content, item.preview, item.annotation].some((value) =>
    String(value || '').toLowerCase().includes(normalizedQuery)
  );
}

export function effectiveItemType(item) {
  if (item?.type === 'link') return 'link';
  if (item?.type !== 'text') return item?.type || 'text';
  return isWebUrl(item.content || item.preview || '') ? 'link' : 'text';
}

export function isWebUrl(value) {
  const trimmed = String(value || '').trim();
  if (!trimmed || /[\s\\\u0000-\u001f\u007f]/.test(trimmed)) return false;

  try {
    const url = new URL(trimmed);
    if (!['http:', 'https:'].includes(url.protocol)) return false;
    if (url.username || url.password) return false;

    const host = url.hostname.replace(/\.$/, '').toLowerCase();
    if (!host || host === 'localhost' || !host.includes('.')) return false;
    if (
      host === '127.0.0.1' ||
      host.startsWith('10.') ||
      host.startsWith('192.168.') ||
      /^172\.(1[6-9]|2\d|3[0-1])\./.test(host) ||
      host === '::1' ||
      host === '[::1]'
    ) {
      return false;
    }

    return true;
  } catch (_e) {
    return false;
  }
}

export function linkDisplayLabel(value) {
  const trimmed = String(value || '').trim();
  try {
    const url = new URL(trimmed);
    const path = `${url.pathname}${url.search}${url.hash}`;
    return path && path !== '/' ? `${url.host}${path}` : url.host;
  } catch (_e) {
    return trimmed;
  }
}

export function isActivationKey(event) {
  return event.key === 'Enter' || event.key === ' ' || event.key === 'Spacebar';
}

export function runKeyboardAction(event, action) {
  if (isActivationKey(event)) {
    event.preventDefault();
    action();
  }
}

export function formatTime(timestamp) {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now - date;

  if (diff < 60000) return '刚刚';
  if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;

  return date.toLocaleString('zh-CN');
}

export function itemLabel(item) {
  if (effectiveItemType(item) === 'link') {
    return item.preview || item.content || '链接记录';
  }

  if (item.type === 'text') {
    return item.preview || item.content || '文本记录';
  }

  if (item.type === 'image') {
    return '图片记录';
  }

  return '剪贴板记录';
}
