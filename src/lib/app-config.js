export const defaultSettings = {
  clipboard_monitor_enabled: true,
  show_main_window_on_start: true,
  auto_start_enabled: false,
  max_items: 50,
  capture_delay_ms: 150,
  screenshot_hotkey: 'CommandOrControl+Shift+A',
  main_window_hotkey: 'CommandOrControl+Shift+Space',
  time_zone: 'Asia/Shanghai',
  language: 'zh-CN',
  auto_cleanup_enabled: false,
  cleanup_max_items: 200,
  cleanup_keep_days: 30,
  dev_server_port: 5174,
};

export const timeZoneOptions = [
  { value: 'Asia/Shanghai', label: '北京（UTC+8）' },
  { value: 'America/New_York', label: '纽约（自动夏令时）' },
  { value: 'Europe/London', label: '伦敦（自动夏令时）' },
  { value: 'Asia/Tokyo', label: '东京（UTC+9）' },
];

export const languageOptions = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en-US', label: 'English' },
];

export const settingsViews = [
  { id: 'basic', label: '常规' },
  { id: 'locale', label: '日期语言' },
  { id: 'advanced', label: '高级' },
  { id: 'about', label: '关于' },
];

export const filters = [
  { id: 'all', label: '全部记录' },
  { id: 'favorite', label: '收藏' },
  { id: 'link', label: '链接' },
  { id: 'image', label: '图片' },
];

export const appVersion = '0.1.7';
export const githubProfileUrl = 'https://github.com/s1oopX';
export const githubRepositoryUrl = 'https://github.com/s1oopX/clipmaster-tauri';
export const githubIssuesUrl = `${githubRepositoryUrl}/issues`;
