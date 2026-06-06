<script>
  import { onDestroy, onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import flatpickr from 'flatpickr';
  import { Mandarin } from 'flatpickr/dist/l10n/zh.js';
  import 'flatpickr/dist/flatpickr.css';
  import {
    Camera,
    CalendarDays,
    Check,
    Clipboard,
    Copy,
    FileText,
    GitPullRequest,
    Heart,
    Image as ImageIcon,
    Inbox,
    List,
    LoaderCircle,
    Pin,
    Search,
    Settings,
    Star,
    Trash2,
    X,
  } from '@lucide/svelte';
  import {
    clipboardApi,
    sessionApi,
    searchApi,
    toolApi,
    settingsApi,
    convertImagePath,
  } from './lib/api.js';

  const defaultSettings = {
    clipboard_monitor_enabled: true,
    show_main_window_on_start: true,
    max_items: 50,
    capture_delay_ms: 150,
    screenshot_hotkey: 'CommandOrControl+Shift+A',
    time_zone: 'Asia/Shanghai',
    language: 'zh-CN',
    auto_cleanup_enabled: false,
    cleanup_max_items: 200,
    cleanup_keep_days: 30,
    dev_server_port: 5174,
  };

  const timeZoneOptions = [
    { value: 'Asia/Shanghai', label: '北京（UTC+8）' },
    { value: 'America/New_York', label: '纽约（自动夏令时）' },
    { value: 'Europe/London', label: '伦敦（自动夏令时）' },
    { value: 'Asia/Tokyo', label: '东京（UTC+9）' },
  ];

  const languageOptions = [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en-US', label: 'English' },
  ];

  const settingsViews = [
    { id: 'basic', label: '常规' },
    { id: 'locale', label: '日期语言' },
    { id: 'advanced', label: '高级' },
    { id: 'about', label: '关于' },
  ];

  const appVersion = '0.1.0';
  const githubProfileUrl = 'https://github.com/s1oopX';
  const githubRepositoryUrl = 'https://github.com/s1oopX/clipmaster-tauri';
  const githubIssuesUrl = `${githubRepositoryUrl}/issues`;

  let items = [];
  let currentSession = null;
  let loading = false;
  let error = null;
  let searchQuery = '';
  let isSearching = false;
  let activeFilter = 'all';
  let filteredItems = [];

  let imageUrls = {};
  let copySuccess = false;
  let copyTimer = null;
  let actionNotice = '';
  let actionError = '';
  let noticeTimer = null;
  let errorNoticeTimer = null;
  let toolLoading = null;
  let settingsOpen = false;
  let activeSettingsView = 'basic';
  let settingsSaving = false;
  let cleanupLoading = false;
  let cleanupPlan = null;
  let portCheckLoading = false;
  let portCheckResult = null;
  let pendingRestartPort = null;
  let restartingApp = false;
  let appSettings = { ...defaultSettings };
  let settingsDraft = { ...defaultSettings };
  let isRecordingHotkey = false;
  let hotkeyRecordingMessage = '';
  let recordingHotkeyTimeout = null;
  let pinMode = false;
  let pinImagePath = '';
  let pinImageUrl = '';
  let unlistenNewItem = null;
  let unlistenHotkey = null;
  let editingId = null;
  let editContent = '';
  let annotationEditingId = null;
  let annotationDraft = '';
  let contextMenu = { open: false, x: 0, y: 0, itemId: null };
  let activeContextItem = null;
  let deleteCandidate = null;
  let deleteConfirmLoading = false;
  let thumbnailUrls = {};
  let viewingImageId = null;
  let availableDays = [];
  let selectedDay = '';
  let recordsScope = '全部日期';
  let recordsRequestId = 0;

  $: activeContextItem = contextMenu.open
    ? items.find((item) => item.id === contextMenu.itemId) || null
    : null;

  const filters = [
    { id: 'all', label: '全部记录' },
    { id: 'favorite', label: '收藏' },
    { id: 'image', label: '图片' },
  ];

  $: filteredItems = activeFilter === 'favorite'
    ? items.filter((item) => item.is_favorite)
    : activeFilter === 'image'
      ? items.filter((item) => item.type === 'image')
      : items;

  $: recordsScope = selectedDay
    || (searchQuery.trim() ? todayDateKey(appSettings.time_zone) : '全部日期');

  $: currentDraftPort = numberSettingValue(
    settingsDraft.dev_server_port,
    defaultSettings.dev_server_port
  );
  $: settingsPortChanged = currentDraftPort
    !== (appSettings.dev_server_port || defaultSettings.dev_server_port);

  function formatDateKey(date) {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  function todayDateKey(timeZone) {
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

  function activeSearchDateKey() {
    return selectedDay || todayDateKey(appSettings.time_zone);
  }

  function itemMatchesSearchQuery(item, query) {
    if (!query) return true;

    const normalizedQuery = query.toLowerCase();
    return [item.content, item.preview, item.annotation].some((value) =>
      String(value || '').toLowerCase().includes(normalizedQuery)
    );
  }

  function itemMatchesLiveScope(item) {
    const query = searchQuery.trim();

    if (selectedDay && item.date_key !== selectedDay) {
      return false;
    }

    if (query && item.date_key !== activeSearchDateKey()) {
      return false;
    }

    return itemMatchesSearchQuery(item, query);
  }

  function datePicker(node, params) {
    let availableDateKeys = new Set(params.availableDays.map((day) => day.date_key));
    let suppressChange = false;

    const picker = flatpickr(node, {
      allowInput: false,
      ariaDateFormat: 'Y年m月d日',
      clickOpens: true,
      dateFormat: 'Y-m-d',
      defaultDate: params.selectedDay || undefined,
      disableMobile: true,
      locale: Mandarin,
      monthSelectorType: 'static',
      nextArrow: '<span class="date-nav-label">下月</span>',
      prevArrow: '<span class="date-nav-label">上月</span>',
      shorthandCurrentMonth: false,
      onChange: (_selectedDates, dateStr) => {
        if (suppressChange) return;
        void selectDay(dateStr);
      },
      onDayCreate: (_dObj, _dStr, _fp, dayElem) => {
        if (availableDateKeys.has(formatDateKey(dayElem.dateObj))) {
          dayElem.classList.add('has-clipboard-items');
        }
      },
    });

    picker.calendarContainer.classList.add('clipmaster-date-picker');
    picker.prevMonthNav?.setAttribute('aria-label', '上个月');
    picker.nextMonthNav?.setAttribute('aria-label', '下个月');

    function sync(nextParams) {
      availableDateKeys = new Set(nextParams.availableDays.map((day) => day.date_key));

      suppressChange = true;
      try {
        if (nextParams.selectedDay) {
          picker.setDate(nextParams.selectedDay, false);
        } else {
          picker.clear(false);
        }

        picker.redraw();
      } finally {
        queueMicrotask(() => {
          suppressChange = false;
        });
      }
    }

    return {
      update: sync,
      destroy() {
        picker.destroy();
      },
    };
  }

  function closeContextMenu() {
    contextMenu = { open: false, x: 0, y: 0, itemId: null };
  }

  function handleDocumentClick(event) {
    if (!contextMenu.open) return;
    if (event.target?.closest?.('.context-menu')) return;
    closeContextMenu();
  }

  function handleDocumentKeyDown(event) {
    if (event.key === 'Escape') {
      closeContextMenu();
    }
  }

  function openContextMenu(event, item) {
    event.preventDefault();

    const estimatedWidth = 178;
    const estimatedHeight = item.type === 'text' ? 188 : 144;
    const x = Math.min(
      Math.max(8, event.clientX),
      Math.max(8, window.innerWidth - estimatedWidth - 8)
    );
    const y = Math.min(
      Math.max(8, event.clientY),
      Math.max(8, window.innerHeight - estimatedHeight - 8)
    );

    contextMenu = { open: true, x, y, itemId: item.id };
  }

  function runContextAction(action) {
    closeContextMenu();
    action();
  }

  function isActivationKey(event) {
    return event.key === 'Enter' || event.key === ' ' || event.key === 'Spacebar';
  }

  function runKeyboardAction(event, action) {
    if (!isActivationKey(event)) return;
    event.preventDefault();
    action();
  }

  function closeImageViewerFromKeyboard(event) {
    if (event.key !== 'Escape' && !isActivationKey(event)) return;
    event.preventDefault();
    closeImageViewer();
  }

  onMount(async () => {
    try {
      const params = new URLSearchParams(window.location.search);
      const pinPath = params.get('pin');

      if (pinPath) {
        pinMode = true;
        pinImagePath = decodeURIComponent(pinPath);
        pinImageUrl = await convertImagePath(pinImagePath);
        return;
      }

      appSettings = {
        ...defaultSettings,
        ...(await settingsApi.getSettings()),
      };
      settingsDraft = { ...appSettings };

      currentSession = await sessionApi.getCurrentSession();
      await loadAvailableDays();
      await refreshVisibleRecords();
      document.addEventListener('click', handleDocumentClick);
      document.addEventListener('keydown', handleDocumentKeyDown);

      // 监听快捷键事件
      unlistenHotkey = await listen('hotkey:screenshot', async () => {
        await startScreenshot();
      });

      unlistenNewItem = await clipboardApi.onNewItem(async (item) => {
        await loadAvailableDays();

        if (itemMatchesLiveScope(item)) {
          items = limitItems([
            item,
            ...items.filter((existing) => existing.id !== item.id),
          ]);
          sortItems();
          reconcileTransientItemState(items);

          if (item.type === 'image' && item.image_path) {
            imageUrls[item.id] = await convertImagePath(item.image_path);
          }

          if (item.type === 'image' && item.thumbnail_path) {
            thumbnailUrls[item.id] = await convertImagePath(item.thumbnail_path);
          }

          pruneImageUrls(items);
        }
      });
    } catch (e) {
      console.error('初始化失败:', e);
      error = e.toString();
    }
  });

  onDestroy(() => {
    if (typeof unlistenNewItem === 'function') {
      unlistenNewItem();
    }

    if (typeof unlistenHotkey === 'function') {
      unlistenHotkey();
    }

    if (copyTimer) clearTimeout(copyTimer);
    if (noticeTimer) clearTimeout(noticeTimer);
    if (errorNoticeTimer) clearTimeout(errorNoticeTimer);
    if (recordingHotkeyTimeout) clearTimeout(recordingHotkeyTimeout);
    document.removeEventListener('click', handleDocumentClick);
    document.removeEventListener('keydown', handleDocumentKeyDown);
  });

  async function loadItems(day = selectedDay) {
    const requestId = ++recordsRequestId;
    isSearching = false;
    loading = true;

    try {
      const nextItems = day
        ? await clipboardApi.getItemsByDay(day, itemLimit(), 0)
        : await clipboardApi.getItems(itemLimit(), 0);

      if (requestId !== recordsRequestId) return;

      items = nextItems;
      error = null;
      actionError = '';
      pruneImageUrls(items);
      reconcileTransientItemState(items);
      void loadImageUrls();
    } catch (e) {
      if (requestId !== recordsRequestId) return;

      console.error('加载记录失败:', e);
      error = e.toString();
    } finally {
      if (requestId === recordsRequestId) {
        loading = false;
      }
    }
  }

  async function loadAvailableDays() {
    try {
      availableDays = await clipboardApi.getAvailableDays(365);
    } catch (e) {
      console.error('加载日期列表失败:', e);
      availableDays = [];
    }
  }

  async function selectDay(day) {
    selectedDay = day;
    searchQuery = '';
    await loadItems(day);
  }

  async function clearDayFilter() {
    await selectDay('');
  }

  async function loadImageUrls() {
    for (const item of items) {
      if (item.type === 'image' && item.image_path && !imageUrls[item.id]) {
        try {
          imageUrls[item.id] = await convertImagePath(item.image_path);
        } catch (e) {
          console.error('加载图片 URL 失败:', e);
        }
      }

      if (item.type === 'image' && item.thumbnail_path && !thumbnailUrls[item.id]) {
        try {
          thumbnailUrls[item.id] = await convertImagePath(item.thumbnail_path);
        } catch (e) {
          console.error('加载缩略图 URL 失败:', e);
        }
      }
    }

    imageUrls = imageUrls;
    thumbnailUrls = thumbnailUrls;
  }

  function requiresDeleteConfirmation(item) {
    return Boolean(item?.is_favorite || item?.annotation);
  }

  function deleteReasonLabel(item) {
    const reasons = [];
    if (item?.is_favorite) reasons.push('已收藏');
    if (item?.annotation) reasons.push('有标注');
    return reasons.join('、');
  }

  async function performDeleteItem(itemId) {
    try {
      await clipboardApi.deleteItem(itemId);
      items = items.filter((item) => item.id !== itemId);
      pruneImageUrls(items);
      reconcileTransientItemState(items);
      await loadAvailableDays();
      error = null;
      showActionNotice('已删除记录');
      return true;
    } catch (e) {
      console.error('删除失败:', e);
      showActionError('删除失败: ' + e);
      return false;
    }
  }

  function requestDeleteItem(item) {
    if (!requiresDeleteConfirmation(item)) {
      void performDeleteItem(item.id);
      return;
    }

    deleteCandidate = item;
  }

  function cancelDeleteConfirmation() {
    if (deleteConfirmLoading) return;
    deleteCandidate = null;
  }

  async function confirmDeleteCandidate() {
    if (!deleteCandidate || deleteConfirmLoading) return;

    deleteConfirmLoading = true;
    const itemId = deleteCandidate.id;

    try {
      const deleted = await performDeleteItem(itemId);
      if (deleted) {
        deleteCandidate = null;
      }
    } finally {
      deleteConfirmLoading = false;
    }
  }

  async function toggleFavorite(itemId) {
    try {
      const isFavorite = await clipboardApi.toggleFavorite(itemId);
      items = items.map((item) =>
        item.id === itemId ? { ...item, is_favorite: isFavorite } : item
      );
      error = null;
      actionError = '';
    } catch (e) {
      console.error('切换收藏失败:', e);
      showActionError('切换收藏失败: ' + e);
    }
  }

  async function togglePinned(itemId) {
    try {
      const isPinned = await clipboardApi.togglePinned(itemId);
      items = items.map((item) =>
        item.id === itemId ? { ...item, is_pinned: isPinned } : item
      );
      sortItems();
      error = null;
      actionError = '';
    } catch (e) {
      console.error('切换置顶失败:', e);
      showActionError('切换置顶失败: ' + e);
    }
  }

  function sortItems() {
    items = [...items].sort((a, b) => {
      if (a.is_pinned && !b.is_pinned) return -1;
      if (!a.is_pinned && b.is_pinned) return 1;
      return b.timestamp - a.timestamp;
    });
  }

  async function handleSearch() {
    const query = searchQuery.trim();

    if (!query) {
      await loadItems();
      return;
    }

    const requestId = ++recordsRequestId;
    loading = false;
    isSearching = true;

    try {
      const dateKey = activeSearchDateKey();
      const nextItems = await searchApi.searchItems(
        query,
        null,
        itemLimit(),
        dateKey
      );

      if (requestId !== recordsRequestId) return;

      items = nextItems;
      error = null;
      actionError = '';
      pruneImageUrls(items);
      reconcileTransientItemState(items);
      void loadImageUrls();
    } catch (e) {
      if (requestId !== recordsRequestId) return;

      console.error('搜索失败:', e);
      error = null;
      showActionError('搜索失败: ' + e);
    } finally {
      if (requestId === recordsRequestId) {
        isSearching = false;
      }
    }
  }

  async function refreshVisibleRecords() {
    if (searchQuery.trim()) {
      await handleSearch();
    } else {
      await loadItems();
    }
  }

  function clearSearch() {
    searchQuery = '';
    loadItems();
  }

  async function copyItem(item) {
    try {
      if (item.type === 'text' && item.content) {
        await clipboardApi.copyToClipboard(item.content);
        error = null;
        showCopyToast();
      } else if (item.type === 'image' && item.image_path) {
        await clipboardApi.copyImageToClipboard(item.image_path);
        error = null;
        showCopyToast();
      } else if (item.type === 'image') {
        showActionError('图片路径不可用');
      }
    } catch (e) {
      console.error('复制失败:', e);
      showActionError('复制失败: ' + e);
    }
  }

  async function startScreenshot() {
    toolLoading = 'screenshot';
    error = null;

    try {
      // 直接进入全屏选取模式
      await toolApi.startRegionScreenshot();
      actionError = '';
      toolLoading = null;
    } catch (e) {
      console.error('截图失败:', e);
      showActionError(normalizeScreenshotError(e));
      toolLoading = null;
    }
  }

  function normalizeScreenshotError(errorValue) {
    const message = String(errorValue || '');

    if (message.includes('screenshot-selector') && message.includes('already exists')) {
      return '截图窗口已打开，请完成当前选区或按 Esc 取消后再试';
    }

    if (message.trim()) {
      return '截图失败: ' + message;
    }

    return '截图失败，请稍后再试';
  }

  async function pinNewestImage() {
    const image = visibleItems().find((item) => item.type === 'image' && item.image_path)
      || items.find((item) => item.type === 'image' && item.image_path);

    if (!image) {
      showActionError('当前没有可钉住的图片记录');
      return;
    }

    await pinImageToDesktop(image);
  }

  async function pinImageToDesktop(item) {
    if (!item.image_path) {
      showActionError('图片路径不可用');
      return;
    }

    toolLoading = 'pin';
    error = null;

    try {
      await toolApi.pinImage(item.image_path);
      showActionNotice('已钉到桌面');
    } catch (e) {
      console.error('贴图失败:', e);
      showActionError('贴图失败: ' + e);
    } finally {
      toolLoading = null;
    }
  }

  function openSettings() {
    settingsDraft = { ...appSettings };
    cleanupPlan = null;
    portCheckResult = null;
    activeSettingsView = 'basic';
    settingsOpen = true;
  }

  async function openExternalLink(event, url) {
    event.preventDefault();

    try {
      await toolApi.openExternalUrl(url);
    } catch (e) {
      console.error('打开链接失败:', e);
      showActionError('打开链接失败: ' + e);
    }
  }

  function updateSettingsDraft(key, value) {
    if (key === 'dev_server_port') {
      portCheckResult = null;
      pendingRestartPort = null;
    }

    settingsDraft = {
      ...settingsDraft,
      [key]: value,
    };
  }

  function optionLabel(options, value) {
    return options.find((option) => option.value === value)?.label || value;
  }

  function numberSettingValue(value, fallback) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  async function checkDevServerPort() {
    portCheckLoading = true;
    portCheckResult = null;
    error = null;

    try {
      portCheckResult = await settingsApi.checkDevServerPort(currentDraftPort);
    } catch (e) {
      console.error('检查端口失败:', e);
      showActionError('检查端口失败: ' + e);
    } finally {
      portCheckLoading = false;
    }
  }

  function applySuggestedPort(port) {
    updateSettingsDraft('dev_server_port', port);
    portCheckResult = {
      port,
      available: true,
      suggested_port: null,
      message: `端口 ${port} 可用`,
    };
  }

  async function restartApplication() {
    restartingApp = true;
    error = null;

    try {
      await settingsApi.restartApp();
    } catch (e) {
      console.error('重启应用失败:', e);
      showActionError('重启应用失败: ' + e);
      restartingApp = false;
    }
  }

  async function saveSettings() {
    settingsSaving = true;
    error = null;

    const normalized = {
      clipboard_monitor_enabled: settingsDraft.clipboard_monitor_enabled,
      show_main_window_on_start: settingsDraft.show_main_window_on_start,
      max_items: numberSettingValue(settingsDraft.max_items, defaultSettings.max_items),
      capture_delay_ms: numberSettingValue(
        settingsDraft.capture_delay_ms,
        defaultSettings.capture_delay_ms
      ),
      screenshot_hotkey: settingsDraft.screenshot_hotkey || defaultSettings.screenshot_hotkey,
      time_zone: settingsDraft.time_zone || defaultSettings.time_zone,
      language: settingsDraft.language || defaultSettings.language,
      auto_cleanup_enabled: settingsDraft.auto_cleanup_enabled,
      cleanup_max_items: numberSettingValue(
        settingsDraft.cleanup_max_items,
        defaultSettings.cleanup_max_items
      ),
      cleanup_keep_days: numberSettingValue(
        settingsDraft.cleanup_keep_days,
        defaultSettings.cleanup_keep_days
      ),
      dev_server_port: numberSettingValue(
        settingsDraft.dev_server_port,
        defaultSettings.dev_server_port
      ),
    };

    try {
      const timeZoneChanged = normalized.time_zone !== appSettings.time_zone;
      const devServerPortChanged = normalized.dev_server_port
        !== (appSettings.dev_server_port || defaultSettings.dev_server_port);
      const savedSettings = await settingsApi.saveSettings(normalized);
      let autoCleanupPlan = null;
      let autoCleanupError = null;

      if (savedSettings.auto_cleanup_enabled) {
        try {
          autoCleanupPlan = await settingsApi.runCustomCleanup(
            savedSettings.cleanup_max_items,
            savedSettings.cleanup_keep_days
          );
        } catch (cleanupError) {
          console.error('自动清理失败:', cleanupError);
          autoCleanupError = cleanupError;
        }
      }

      appSettings = savedSettings;
      settingsDraft = { ...appSettings };
      cleanupPlan = autoCleanupPlan;
      if (devServerPortChanged) {
        pendingRestartPort = savedSettings.dev_server_port;
        activeSettingsView = 'advanced';
        settingsOpen = true;
      } else {
        settingsOpen = false;
      }
      if (timeZoneChanged) {
        selectedDay = '';
      }
      await loadAvailableDays();
      await refreshVisibleRecords();

      if (autoCleanupError) {
        showActionError('设置已保存，自动清理失败: ' + autoCleanupError);
      } else if (autoCleanupPlan) {
        showActionNotice(`设置已保存，已清理 ${autoCleanupPlan.item_count} 条记录`);
      } else if (devServerPortChanged) {
        showActionNotice('端口已保存，重启后生效');
      } else {
        showActionNotice('设置已保存');
      }
    } catch (e) {
      console.error('保存设置失败:', e);
      showActionError('保存设置失败: ' + e);
    } finally {
      settingsSaving = false;
    }
  }

  async function previewCleanup() {
    cleanupLoading = true;
    error = null;

    try {
      cleanupPlan = await settingsApi.previewCustomCleanup(
        numberSettingValue(settingsDraft.cleanup_max_items, defaultSettings.cleanup_max_items),
        numberSettingValue(settingsDraft.cleanup_keep_days, defaultSettings.cleanup_keep_days)
      );
    } catch (e) {
      console.error('预览清理失败:', e);
      showActionError('预览清理失败: ' + e);
    } finally {
      cleanupLoading = false;
    }
  }

  async function runCleanupNow() {
    cleanupLoading = true;
    error = null;

    try {
      cleanupPlan = await settingsApi.runCustomCleanup(
        numberSettingValue(settingsDraft.cleanup_max_items, defaultSettings.cleanup_max_items),
        numberSettingValue(settingsDraft.cleanup_keep_days, defaultSettings.cleanup_keep_days)
      );
      await loadAvailableDays();
      await refreshVisibleRecords();
      showActionNotice(`已清理 ${cleanupPlan.item_count} 条记录`);
    } catch (e) {
      console.error('执行清理失败:', e);
      showActionError('执行清理失败: ' + e);
    } finally {
      cleanupLoading = false;
    }
  }

  async function closePinWindow() {
    try {
      await getCurrentWindow().close();
    } catch (e) {
      console.error('关闭贴图窗口失败:', e);
      window.close();
    }
  }

  function showCopyToast() {
    copySuccess = true;
    actionNotice = '';
    actionError = '';
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      copySuccess = false;
    }, 1800);
  }

  function showActionNotice(message) {
    actionNotice = message;
    copySuccess = false;
    actionError = '';
    if (noticeTimer) clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      actionNotice = '';
    }, 2200);
  }

  function showActionError(message) {
    actionError = message;
    copySuccess = false;
    actionNotice = '';
    if (errorNoticeTimer) clearTimeout(errorNoticeTimer);
    errorNoticeTimer = setTimeout(() => {
      actionError = '';
    }, 3200);
  }

  // 快捷键录制相关
  function startRecordingHotkey() {
    isRecordingHotkey = true;
    hotkeyRecordingMessage = '';

    // 清除之前的超时
    if (recordingHotkeyTimeout) {
      clearTimeout(recordingHotkeyTimeout);
    }

    // 5秒后自动停止录制
    recordingHotkeyTimeout = setTimeout(() => {
      stopRecordingHotkey();
    }, 5000);
  }

  function stopRecordingHotkey() {
    isRecordingHotkey = false;
    hotkeyRecordingMessage = '';
    if (recordingHotkeyTimeout) {
      clearTimeout(recordingHotkeyTimeout);
      recordingHotkeyTimeout = null;
    }
  }

  function handleHotkeyKeyDown(event) {
    if (!isRecordingHotkey) return;

    event.preventDefault();

    // 忽略单独的修饰键
    if (['Control', 'Shift', 'Alt', 'Meta', 'Command'].includes(event.key)) {
      return;
    }

    // 构建快捷键字符串
    const parts = [];

    // 跨平台：使用 CommandOrControl
    if (event.ctrlKey || event.metaKey) {
      parts.push('CommandOrControl');
    }

    if (event.altKey) {
      parts.push('Alt');
    }

    if (event.shiftKey) {
      parts.push('Shift');
    }

    // 添加主键
    let key = event.key.toUpperCase();

    // 标准化某些特殊键
    const keyMap = {
      ' ': 'Space',
      'ARROWUP': 'Up',
      'ARROWDOWN': 'Down',
      'ARROWLEFT': 'Left',
      'ARROWRIGHT': 'Right',
    };

    key = keyMap[key] || key;

    // 必须有修饰键
    if (parts.length === 0) {
      hotkeyRecordingMessage = '请使用修饰键组合（如 Ctrl+Shift+A）';
      return;
    }

    parts.push(key);

    const hotkey = parts.join('+');
    updateSettingsDraft('screenshot_hotkey', hotkey);
    hotkeyRecordingMessage = '';

    // 停止录制
    stopRecordingHotkey();
  }

  function visibleItems() {
    return filteredItems;
  }

  function itemLimit() {
    return appSettings.max_items || defaultSettings.max_items;
  }

  function limitItems(nextItems) {
    return nextItems.slice(0, itemLimit());
  }

  function pruneImageUrls(nextItems = items) {
    const liveIds = new Set(nextItems.map((item) => item.id));

    imageUrls = Object.fromEntries(
      Object.entries(imageUrls).filter(([itemId]) => liveIds.has(itemId))
    );

    thumbnailUrls = Object.fromEntries(
      Object.entries(thumbnailUrls).filter(([itemId]) => liveIds.has(itemId))
    );
  }

  function reconcileTransientItemState(nextItems = items) {
    const liveIds = new Set(nextItems.map((item) => item.id));

    if (contextMenu.open && !liveIds.has(contextMenu.itemId)) {
      closeContextMenu();
    }

    if (deleteCandidate && !liveIds.has(deleteCandidate.id)) {
      deleteCandidate = null;
      deleteConfirmLoading = false;
    }

    if (editingId && !liveIds.has(editingId)) {
      cancelContentEdit();
    }

    if (annotationEditingId && !liveIds.has(annotationEditingId)) {
      cancelAnnotationEdit();
    }

    if (viewingImageId && !liveIds.has(viewingImageId)) {
      closeImageViewer();
    }
  }

  function updateVisibleItem(itemId, updater) {
    const nextItems = items.map((item) =>
      item.id === itemId ? updater(item) : item
    );

    items = searchQuery.trim()
      ? nextItems.filter(itemMatchesLiveScope)
      : nextItems;
    pruneImageUrls(items);
    reconcileTransientItemState(items);
  }

  function startContentEdit(item) {
    if (item.type !== 'text') return;
    editingId = item.id;
    editContent = item.content || '';
    annotationEditingId = null;
    annotationDraft = '';
  }

  function cancelContentEdit() {
    editingId = null;
    editContent = '';
  }

  function startAnnotationEdit(item) {
    annotationEditingId = item.id;
    annotationDraft = item.annotation || '';
    editingId = null;
    editContent = '';
  }

  function cancelAnnotationEdit() {
    annotationEditingId = null;
    annotationDraft = '';
  }

  function viewFullImage(itemId) {
    viewingImageId = itemId;
  }

  function closeImageViewer() {
    viewingImageId = null;
  }

  async function saveContentEdit(itemId) {
    if (!editContent.trim()) {
      showActionError('原文不能为空');
      return;
    }

    try {
      await clipboardApi.updateItemContent(itemId, editContent);

      updateVisibleItem(itemId, (item) => ({
        ...item,
        content: editContent,
        preview: editContent.length > 100
          ? editContent.substring(0, 100) + '...'
          : editContent,
      }));

      editingId = null;
      editContent = '';
      showActionNotice('原文已更新');
    } catch (e) {
      console.error('保存原文失败:', e);
      showActionError('保存原文失败: ' + e);
    }
  }

  async function saveAnnotation(itemId) {
    try {
      const savedAnnotation = await clipboardApi.updateItemAnnotation(itemId, annotationDraft);

      updateVisibleItem(itemId, (item) => ({
        ...item,
        annotation: savedAnnotation,
      }));

      annotationEditingId = null;
      annotationDraft = '';
      showActionNotice(savedAnnotation ? '标注已保存' : '标注已清除');
    } catch (e) {
      console.error('保存标注失败:', e);
      showActionError('保存标注失败: ' + e);
    }
  }

  function formatTime(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;

    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;

    return date.toLocaleString('zh-CN');
  }

  function itemLabel(item) {
    if (item.type === 'text') {
      return item.preview || item.content || '文本记录';
    }

    if (item.type === 'image') {
      return '图片记录';
    }

    return '剪贴板记录';
  }
</script>

{#if pinMode}
  <main class="pin-shell" data-testid="pin-shell">
    <header class="pin-toolbar" data-tauri-drag-region>
      <span>ClipMaster 贴图</span>
      <button type="button" on:click={closePinWindow} aria-label="关闭贴图">
        <X size={15} aria-hidden="true" />
      </button>
    </header>

    <section class="pin-image-stage" aria-label="桌面贴图">
      {#if pinImageUrl}
        <img src={pinImageUrl} alt="桌面贴图" decoding="async" />
      {:else}
        <div class="image-loading">{pinImagePath || '图片加载中'}</div>
      {/if}
    </section>
  </main>
{:else}
<main class="app-shell" data-testid="app-shell" data-layout="compact-ready" data-density="tool">
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">
        <Clipboard size={20} aria-hidden="true" />
      </div>
      <div>
        <h1>ClipMaster</h1>
        <p>快速剪存与回放</p>
      </div>
    </div>

    <nav class="filter-nav" aria-label="剪贴板筛选">
      {#each filters as filter}
        <button
          class="filter-button"
          class:active={activeFilter === filter.id}
          on:click={() => (activeFilter = filter.id)}
          type="button"
        >
          {#if filter.id === 'all'}
            <List size={16} aria-hidden="true" />
          {:else if filter.id === 'favorite'}
            <Heart size={16} aria-hidden="true" />
          {:else}
            <ImageIcon size={16} aria-hidden="true" />
          {/if}
          <span>{filter.label}</span>
        </button>
      {/each}
    </nav>

  </aside>

  <section class="workspace" aria-label="剪贴板历史">
    <header class="toolbar">
      <div class="toolbar-title">
        <p class="eyebrow">History</p>
        <div class="toolbar-heading">
          <h2>剪贴板历史</h2>
          <p class="toolbar-context" aria-label="当前范围">
            <span class="status-dot"></span>
            {recordsScope} · {filteredItems.length} 条
          </p>
        </div>
      </div>

      <div class="toolbar-tools">
        <div class="toolbar-primary">
          <div class="quick-actions" aria-label="快速工具">
            <button
              type="button"
              class="tool-button"
              on:click={startScreenshot}
              disabled={toolLoading === 'screenshot'}
            >
              {#if toolLoading === 'screenshot'}
                <LoaderCircle size={15} aria-hidden="true" />
              {:else}
                <Camera size={15} aria-hidden="true" />
              {/if}
              <span>截图</span>
            </button>

            <button
              type="button"
              class="tool-button"
              on:click={pinNewestImage}
              disabled={toolLoading === 'pin'}
            >
              <Pin size={15} aria-hidden="true" />
              <span>钉住</span>
            </button>

            <button type="button" class="icon-tool" on:click={openSettings} aria-label="设置">
              <Settings size={17} aria-hidden="true" />
            </button>
          </div>
        </div>

        <div class="toolbar-secondary">
          <div class="day-field calendar-field">
            <CalendarDays size={15} aria-hidden="true" />
            <input
              id="day-picker"
              type="text"
              value={selectedDay}
              placeholder="全部日期"
              readonly
              use:datePicker={{ selectedDay, availableDays }}
              aria-label="按日期精确选择剪贴板记录"
            />
            {#if selectedDay}
              <button type="button" class="clear-date" on:click={clearDayFilter} aria-label="清除日期筛选">
                <X size={14} aria-hidden="true" />
              </button>
            {/if}
          </div>

          {#if availableDays.length > 0}
            <div class="date-shortcuts" aria-label="有记录的日期快捷选择">
              {#each availableDays.slice(0, 3) as day}
                <button
                  type="button"
                  class:active={selectedDay === day.date_key}
                  on:click={() => selectDay(day.date_key)}
                >
                  {day.date_key.slice(5)} · {day.item_count}
                </button>
              {/each}
            </div>
          {/if}

          <label class="search-field">
            <Search size={17} aria-hidden="true" />
            <span class="sr-only">搜索剪贴板内容</span>
            <input
              type="search"
              aria-label="搜索剪贴板内容"
              placeholder="搜索内容"
              bind:value={searchQuery}
              on:input={handleSearch}
            />
            {#if searchQuery}
              <button type="button" class="clear-search" on:click={clearSearch} aria-label="清除搜索">
                <X size={15} aria-hidden="true" />
              </button>
            {/if}
          </label>
        </div>
      </div>
    </header>

    {#if error}
      <div class="notice error" role="alert">{error}</div>
    {/if}

    <div class="history-panel" data-testid="history-panel" data-scroll="internal">
      {#if loading || isSearching}
        <div class="loading-stack" role="status" aria-label="加载中">
          <div class="loading-head">
            <LoaderCircle size={18} aria-hidden="true" />
            <span>加载中</span>
          </div>
          {#each Array(4) as _}
            <div class="skeleton-item">
              <span class="skeleton-meta"></span>
              <span class="skeleton-line wide"></span>
              <span class="skeleton-line"></span>
            </div>
          {/each}
        </div>
      {:else if filteredItems.length === 0}
        <div class="empty-state">
          <div class="empty-mark">
            <Inbox size={34} aria-hidden="true" />
          </div>
          {#if searchQuery}
            <h3>未找到匹配的记录</h3>
            <p>换个关键词再试一次。</p>
          {:else}
            <h3>暂无剪贴板记录</h3>
            <p>复制内容后会自动出现在这里</p>
          {/if}
        </div>
      {:else}
        <div class="items-list" aria-label="剪贴板记录列表">
          {#each filteredItems as item (item.id)}
            <article
              class="item"
              class:pinned={item.is_pinned}
              on:contextmenu={(event) => openContextMenu(event, item)}
            >
              <div class="item-main">
                <div class="item-row">
                  <div class="item-meta">
                    <span class="type-pill">
                      {#if item.type === 'image'}
                        <ImageIcon size={14} aria-hidden="true" />
                        图片
                      {:else if item.type === 'file'}
                        <FileText size={14} aria-hidden="true" />
                        文件
                      {:else}
                        <FileText size={14} aria-hidden="true" />
                        文本
                      {/if}
                    </span>
                    <span>{formatTime(item.timestamp)}</span>
                    {#if item.is_pinned}
                      <span class="badge">置顶</span>
                    {/if}
                    {#if item.is_favorite}
                      <span class="badge">收藏</span>
                    {/if}
                    {#if item.annotation}
                      <span class="badge">已标注</span>
                    {/if}
                  </div>

                  <div class="item-actions">
                    <button
                      type="button"
                      class="item-action primary-action"
                      on:click={() => copyItem(item)}
                      aria-label={`复制 ${itemLabel(item)}`}
                    >
                      <Copy size={16} aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      class="item-action secondary-action"
                      class:active={item.is_pinned}
                      on:click={() => togglePinned(item.id)}
                      aria-label={`置顶 ${itemLabel(item)}`}
                    >
                      <Pin size={16} aria-hidden="true" />
                    </button>
                    {#if item.type === 'image' && item.image_path}
                      <button
                        type="button"
                        class="item-action secondary-action"
                        on:click={() => pinImageToDesktop(item)}
                        aria-label={`钉到桌面 ${itemLabel(item)}`}
                      >
                        <Pin size={16} aria-hidden="true" />
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="item-action secondary-action"
                      class:active={annotationEditingId === item.id}
                      on:click={() => startAnnotationEdit(item)}
                      aria-label={`标注 ${itemLabel(item)}`}
                    >
                      <FileText size={16} aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      class="item-action primary-action"
                      class:active={item.is_favorite}
                      on:click={() => toggleFavorite(item.id)}
                      aria-label={`收藏 ${itemLabel(item)}`}
                    >
                      <Star size={16} aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      class="item-action secondary-action danger-action"
                      on:click={() => requestDeleteItem(item)}
                      aria-label={`删除 ${itemLabel(item)}`}
                    >
                      <Trash2 size={16} aria-hidden="true" />
                    </button>
                  </div>
                </div>

                {#if item.type === 'text'}
                  {#if editingId === item.id}
                    <div class="edit-area">
                      <textarea
                        bind:value={editContent}
                        placeholder="编辑原始文本内容"
                        rows="4"
                        aria-label={`编辑 ${itemLabel(item)} 的原文`}
                      ></textarea>
                      <div class="edit-actions">
                        <button
                          type="button"
                          class="btn-save"
                          on:click={() => saveContentEdit(item.id)}
                        >
                          <Check size={16} aria-hidden="true" />
                          保存原文
                        </button>
                        <button
                          type="button"
                          class="btn-cancel"
                          on:click={cancelContentEdit}
                        >
                          <X size={16} aria-hidden="true" />
                          取消
                        </button>
                      </div>
                    </div>
                  {:else}
                    <div
                      class="text-content copyable"
                      role="button"
                      tabindex="0"
                      on:dblclick={(event) => {
                        event.preventDefault();
                        copyItem(item);
                      }}
                      on:keydown={(event) => {
                        runKeyboardAction(event, () => copyItem(item));
                      }}
                    >
                      {item.preview || item.content}
                    </div>
                  {/if}
                {:else if item.type === 'image'}
                  <div class="image-summary">
                    <strong>图片记录</strong>
                    <span>{item.image_path || '等待图片路径'}</span>
                  </div>
                  {#if thumbnailUrls[item.id]}
                    <div
                      class="image-preview"
                      on:click={() => viewFullImage(item.id)}
                      role="button"
                      tabindex="0"
                      on:keydown={(event) => runKeyboardAction(event, () => viewFullImage(item.id))}
                    >
                      <img
                        src={thumbnailUrls[item.id]}
                        alt="剪贴板图片缩略图"
                        loading="lazy"
                        decoding="async"
                        on:error={(event) => {
                          console.error('缩略图加载失败:', item.thumbnail_path);
                          event.target.style.display = 'none';
                        }}
                      />
                    </div>
                  {:else}
                    <div class="image-loading">图片加载中</div>
                  {/if}
                {/if}

                {#if annotationEditingId === item.id}
                  <div class="annotation-editor">
                    <textarea
                      bind:value={annotationDraft}
                      placeholder="添加标注，不会改变原始内容"
                      rows="3"
                      aria-label={`编辑 ${itemLabel(item)} 的标注`}
                    ></textarea>
                    <div class="edit-actions">
                      <button
                        type="button"
                        class="btn-save"
                        on:click={() => saveAnnotation(item.id)}
                      >
                        <Check size={16} aria-hidden="true" />
                        保存标注
                      </button>
                      <button
                        type="button"
                        class="btn-cancel"
                        on:click={cancelAnnotationEdit}
                      >
                        <X size={16} aria-hidden="true" />
                        取消
                      </button>
                    </div>
                  </div>
                {:else if item.annotation}
                  <div class="annotation-note">
                    <span>标注</span>
                    <p>{item.annotation}</p>
                  </div>
                {/if}
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>

    {#if copySuccess || actionNotice || actionError}
      <div class="toast-stack" data-testid="toast-stack" aria-live="polite">
        {#if copySuccess}
          <div class="toast success" role="status">
            <Check size={16} aria-hidden="true" />
            <span>已复制到剪贴板</span>
          </div>
        {/if}

        {#if actionNotice}
          <div class="toast success" role="status">
            <Check size={16} aria-hidden="true" />
            <span>{actionNotice}</span>
          </div>
        {/if}

        {#if actionError}
          <div class="toast error" role="alert">
            <X size={16} aria-hidden="true" />
            <span>{actionError}</span>
          </div>
        {/if}
      </div>
    {/if}
  </section>
  {#if contextMenu.open && activeContextItem}
    <div
      class="context-menu"
      role="menu"
      tabindex="-1"
      style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
    >
      <button
        type="button"
        role="menuitem"
        on:click={() => runContextAction(() => copyItem(activeContextItem))}
      >
        <Copy size={15} aria-hidden="true" />
        复制
      </button>
      {#if activeContextItem.type === 'text'}
        <button
          type="button"
          role="menuitem"
          on:click={() => runContextAction(() => startContentEdit(activeContextItem))}
        >
          <FileText size={15} aria-hidden="true" />
          编辑原文
        </button>
      {/if}
      <button
        type="button"
        role="menuitem"
        on:click={() => runContextAction(() => startAnnotationEdit(activeContextItem))}
      >
        <FileText size={15} aria-hidden="true" />
        {activeContextItem.annotation ? '编辑标注' : '添加标注'}
      </button>
    </div>
  {/if}
  {#if settingsOpen}
    <div class="settings-backdrop" on:click={() => (settingsOpen = false)} aria-hidden="true"></div>
    <div
      class="settings-panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <header class="settings-header">
        <div>
          <p class="eyebrow">Preferences</p>
          <h2 id="settings-title">设置</h2>
        </div>
        <button type="button" on:click={() => (settingsOpen = false)} aria-label="关闭设置">
          <X size={16} aria-hidden="true" />
        </button>
      </header>

      <div class="settings-workspace">
        <div class="settings-nav" aria-label="设置分类" role="tablist">
          {#each settingsViews as view}
            <button
              type="button"
              role="tab"
              id={`settings-tab-${view.id}`}
              class:active={activeSettingsView === view.id}
              aria-selected={activeSettingsView === view.id}
              aria-controls={`settings-view-${view.id}`}
              on:click={() => (activeSettingsView = view.id)}
            >
              {#if view.id === 'basic'}
                <Settings class="settings-tab-icon" size={15} aria-hidden="true" />
              {:else if view.id === 'locale'}
                <CalendarDays class="settings-tab-icon" size={15} aria-hidden="true" />
              {:else if view.id === 'advanced'}
                <Settings class="settings-tab-icon" size={15} aria-hidden="true" />
              {:else}
                <Clipboard class="settings-tab-icon" size={15} aria-hidden="true" />
              {/if}
              <span>{view.label}</span>
            </button>
          {/each}
        </div>

        <div class="settings-content">
          {#if activeSettingsView === 'basic'}
            <div
              class="settings-section settings-view"
              id="settings-view-basic"
              role="tabpanel"
              aria-labelledby="settings-tab-basic"
            >
              <div class="settings-section-title">
                <h3>常规设置</h3>
                <p>启动 / 监听 / 截图</p>
              </div>

              <label class="switch-row">
                <input
                  type="checkbox"
                  checked={settingsDraft.clipboard_monitor_enabled}
                  on:change={(event) =>
                    updateSettingsDraft('clipboard_monitor_enabled', event.currentTarget.checked)}
                />
                <span>监听剪贴板</span>
              </label>

              <label class="switch-row">
                <input
                  type="checkbox"
                  checked={settingsDraft.show_main_window_on_start}
                  on:change={(event) =>
                    updateSettingsDraft('show_main_window_on_start', event.currentTarget.checked)}
                />
                <span>启动时显示主窗口</span>
              </label>

              <label class="field-row">
                <span>保留记录数</span>
                <input
                  type="number"
                  min="10"
                  max="500"
                  value={settingsDraft.max_items}
                  on:input={(event) =>
                    updateSettingsDraft('max_items', Number(event.currentTarget.value))}
                />
              </label>

              <label class="field-row">
                <span>截图延迟</span>
                <input
                  type="number"
                  min="0"
                  max="3000"
                  step="50"
                  value={settingsDraft.capture_delay_ms}
                  on:input={(event) =>
                    updateSettingsDraft('capture_delay_ms', Number(event.currentTarget.value))}
                />
              </label>

              <div class="settings-section-title inline-section-title">
                <h3>快捷键</h3>
                <p>截图入口</p>
              </div>

              <label class="field-row">
                <span>截图</span>
                <input
                  type="text"
                  readonly
                  placeholder="点击后按下组合键"
                  value={settingsDraft.screenshot_hotkey}
                  on:focus={startRecordingHotkey}
                  on:blur={stopRecordingHotkey}
                  on:keydown={handleHotkeyKeyDown}
                  class:recording={isRecordingHotkey}
                />
              </label>
              <p class="hotkey-hint" aria-live="polite">
                {#if isRecordingHotkey}
                  {hotkeyRecordingMessage || '正在录制，请按下组合键（如 Ctrl+Shift+A）'}
                {:else}
                  点击输入框后按下组合键自动录制，例如 Ctrl+Shift+A
                {/if}
              </p>
            </div>
          {:else if activeSettingsView === 'locale'}
            <div
              class="settings-section settings-view"
              id="settings-view-locale"
              role="tabpanel"
              aria-labelledby="settings-tab-locale"
            >
              <div class="settings-section-title">
                <h3>界面与日期</h3>
                <p>语言 / 自然日</p>
              </div>

              <label class="field-row">
                <span>日期划分时区</span>
                <select
                  value={settingsDraft.time_zone}
                  on:change={(event) => updateSettingsDraft('time_zone', event.currentTarget.value)}
                >
                  {#each timeZoneOptions as option}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>

              <label class="field-row">
                <span>应用语言</span>
                <select
                  value={settingsDraft.language}
                  on:change={(event) => updateSettingsDraft('language', event.currentTarget.value)}
                >
                  {#each languageOptions as option}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
            </div>
          {:else if activeSettingsView === 'advanced'}
            <div
              class="settings-section settings-view"
              id="settings-view-advanced"
              role="tabpanel"
              aria-labelledby="settings-tab-advanced"
            >
              <div class="settings-section-title">
                <h3>高级设置</h3>
                <p>清理 / 开发端口</p>
              </div>

              <div class="settings-section-title inline-section-title first-inline-section-title">
                <h3>记录清理</h3>
                <p>规则 / 预览 / 执行</p>
              </div>

              <label class="switch-row">
                <input
                  type="checkbox"
                  checked={settingsDraft.auto_cleanup_enabled}
                  on:change={(event) =>
                    updateSettingsDraft('auto_cleanup_enabled', event.currentTarget.checked)}
                />
                <span>保存设置后自动清理</span>
              </label>

              <label class="field-row">
                <span>普通记录最多保留</span>
                <input
                  type="number"
                  min="10"
                  max="5000"
                  value={settingsDraft.cleanup_max_items}
                  on:input={(event) =>
                    updateSettingsDraft('cleanup_max_items', Number(event.currentTarget.value))}
                />
              </label>

              <label class="field-row">
                <span>普通记录保留天数</span>
                <input
                  type="number"
                  min="1"
                  max="3650"
                  value={settingsDraft.cleanup_keep_days}
                  on:input={(event) =>
                    updateSettingsDraft('cleanup_keep_days', Number(event.currentTarget.value))}
                />
              </label>

              <p class="cleanup-hint">清理仅影响未置顶、未收藏的普通记录；图片文件会同步删除。</p>

              {#if cleanupPlan}
                <p class="cleanup-plan" role="status">
                  将清理 {cleanupPlan.item_count} 条记录（文本 {cleanupPlan.text_count}，图片 {cleanupPlan.image_count}）
                </p>
              {/if}

              <div class="cleanup-actions">
                <button type="button" class="ghost-button" on:click={previewCleanup} disabled={cleanupLoading}>
                  {cleanupLoading ? '计算中' : '预览清理'}
                </button>
                <button type="button" class="ghost-button" on:click={runCleanupNow} disabled={cleanupLoading}>
                  {cleanupLoading ? '清理中' : '立即清理'}
                </button>
              </div>

              <div class="settings-section-title inline-section-title">
                <h3>开发端口</h3>
                <p>占用检测 / 重启生效</p>
              </div>

              <div class="field-row port-field">
                <label for="dev-server-port">开发端口</label>
                <div class="port-input-group">
                  <input
                    id="dev-server-port"
                    type="number"
                    min="1"
                    max="65535"
                    value={settingsDraft.dev_server_port}
                    on:input={(event) =>
                      updateSettingsDraft('dev_server_port', Number(event.currentTarget.value))}
                  />
                  <button
                    type="button"
                    class="ghost-button compact-button"
                    on:click={checkDevServerPort}
                    disabled={portCheckLoading}
                  >
                    {portCheckLoading ? '检查中' : '检查端口'}
                  </button>
                </div>
              </div>

              <div
                class:available={portCheckResult?.available}
                class:occupied={portCheckResult && !portCheckResult.available}
                class="port-check-result"
                aria-live="polite"
              >
                {#if portCheckResult}
                  <p>{portCheckResult.message}</p>
                  {#if !portCheckResult.available && portCheckResult.suggested_port}
                    <button
                      type="button"
                      class="ghost-button compact-button"
                      on:click={() => applySuggestedPort(portCheckResult.suggested_port)}
                    >
                      使用 {portCheckResult.suggested_port}
                    </button>
                  {/if}
                {:else}
                  <p>用于本地开发服务；生产版不占用端口。</p>
                {/if}
              </div>

              {#if settingsPortChanged}
                <p class="port-hint">端口变化需要保存并重启应用后生效。</p>
              {/if}

              {#if pendingRestartPort}
                <div class="restart-card" role="status">
                  <div>
                    <strong>端口 {pendingRestartPort} 已保存</strong>
                    <span>重启后应用会切到新的开发端口。</span>
                  </div>
                  <button
                    type="button"
                    class="primary-button"
                    on:click={restartApplication}
                    disabled={restartingApp}
                  >
                    {restartingApp ? '重启中' : '重启应用'}
                  </button>
                </div>
              {/if}
            </div>
          {:else}
            <div
              class="settings-section settings-view about-section"
              id="settings-view-about"
              role="tabpanel"
              aria-labelledby="settings-tab-about"
            >
              <div class="about-profile">
                <img class="about-avatar" src="/github-avatar-display.jpg" alt="s1oopX GitHub 头像" />
                <div class="about-profile-copy">
                  <span class="about-eyebrow">GitHub · s1oopX</span>
                  <h3>s1oopX</h3>
                  <p>
                    ClipMaster 的作者与维护者。这个工具保持轻巧，只处理复制、截图、标注和贴图这些基础事情。
                  </p>
                </div>
              </div>

              <div class="about-block">
                <h4>项目简介</h4>
                <p>
                  ClipMaster 是一个轻巧的本地剪贴板工具，用来保存复制记录、截图、基础标注和贴图。数据默认保存在本机，记录按日期整理。
                </p>
              </div>

              <dl class="about-list">
                <div>
                  <dt>版本</dt>
                  <dd>{appVersion}</dd>
                </div>
                <div>
                  <dt>数据</dt>
                  <dd>本地保存</dd>
                </div>
                <div>
                  <dt>日期规则</dt>
                  <dd>{optionLabel(timeZoneOptions, settingsDraft.time_zone)}</dd>
                </div>
              </dl>

              <div class="about-block about-contact">
                <h4>联系方式</h4>
                <div class="about-links">
                  <a
                    class="about-link"
                    href={githubProfileUrl}
                    target="_blank"
                    rel="noreferrer"
                    on:click={(event) => openExternalLink(event, githubProfileUrl)}
                  >
                    <GitPullRequest size={14} aria-hidden="true" />
                    <span>
                      <strong>GitHub 主页</strong>
                      <small>s1oopX</small>
                    </span>
                  </a>
                  <a
                    class="about-link"
                    href={githubIssuesUrl}
                    target="_blank"
                    rel="noreferrer"
                    on:click={(event) => openExternalLink(event, githubIssuesUrl)}
                  >
                    <GitPullRequest size={14} aria-hidden="true" />
                    <span>
                      <strong>提交问题或建议</strong>
                      <small>s1oopX/clipmaster-tauri</small>
                    </span>
                  </a>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>

      <footer class="settings-footer">
        <button type="button" class="ghost-button" on:click={() => (settingsOpen = false)}>
          取消
        </button>
        <button type="button" class="primary-button" on:click={saveSettings} disabled={settingsSaving}>
          {settingsSaving ? '保存中' : '保存设置'}
        </button>
      </footer>
    </div>
  {/if}

  {#if deleteCandidate}
    <button
      type="button"
      class="confirm-backdrop"
      aria-label="取消删除确认"
      on:click={cancelDeleteConfirmation}
    ></button>
    <div
      class="confirm-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-confirm-title"
      aria-describedby="delete-confirm-desc"
    >
      <header>
        <div class="confirm-icon">
          <Trash2 size={18} aria-hidden="true" />
        </div>
        <div>
          <h2 id="delete-confirm-title">确认删除</h2>
          <p id="delete-confirm-desc">
            这条记录{deleteReasonLabel(deleteCandidate)}，删除后无法恢复。
          </p>
        </div>
      </header>
      <div class="confirm-preview">
        {itemLabel(deleteCandidate)}
      </div>
      <footer>
        <button
          type="button"
          class="ghost-button"
          on:click={cancelDeleteConfirmation}
          disabled={deleteConfirmLoading}
        >
          取消
        </button>
        <button
          type="button"
          class="danger-button"
          on:click={confirmDeleteCandidate}
          disabled={deleteConfirmLoading}
        >
          {deleteConfirmLoading ? '删除中' : '确认删除'}
        </button>
      </footer>
    </div>
  {/if}

  <!-- 原图查看器 -->
  {#if viewingImageId && imageUrls[viewingImageId]}
    <div
      class="image-viewer-overlay"
      on:click={closeImageViewer}
      role="button"
      tabindex="0"
      on:keydown={closeImageViewerFromKeyboard}
    >
      <div class="image-viewer-content" on:click|stopPropagation role="presentation">
        <button class="image-viewer-close" on:click={closeImageViewer} aria-label="关闭">
          <X size={24} aria-hidden="true" />
        </button>
        <img
          src={imageUrls[viewingImageId]}
          alt="原图"
          on:error={(event) => {
            console.error('原图加载失败');
            closeImageViewer();
          }}
        />
      </div>
    </div>
  {/if}
</main>
{/if}

<style>
  :global(body) {
    margin: 0;
    background: #f4f6f8;
    color: #172033;
    font-family:
      Inter,
      ui-sans-serif,
      system-ui,
      -apple-system,
      BlinkMacSystemFont,
      'Segoe UI',
      sans-serif;
  }

  :global(button),
  :global(input) {
    font: inherit;
  }

  .pin-shell {
    display: grid;
    grid-template-rows: 34px minmax(0, 1fr);
    width: 100%;
    height: 100vh;
    overflow: hidden;
    color: #e5edf7;
    background: #0b1120;
  }

  .pin-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 0 8px 0 12px;
    color: #dbeafe;
    background: rgba(8, 13, 26, 0.92);
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
    font-size: 0.78rem;
    font-weight: 650;
    user-select: none;
  }

  .pin-toolbar button {
    display: grid;
    width: 24px;
    height: 24px;
    place-items: center;
    color: #cbd5e1;
    background: transparent;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
  }

  .pin-toolbar button:hover {
    color: #ffffff;
    background: rgba(148, 163, 184, 0.18);
  }

  .pin-image-stage {
    display: grid;
    min-width: 0;
    min-height: 0;
    place-items: center;
    padding: 8px;
    background:
      linear-gradient(45deg, rgba(148, 163, 184, 0.09) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(148, 163, 184, 0.09) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, rgba(148, 163, 184, 0.09) 75%),
      linear-gradient(-45deg, transparent 75%, rgba(148, 163, 184, 0.09) 75%),
      #0b1120;
    background-position:
      0 0,
      0 8px,
      8px -8px,
      -8px 0;
    background-size: 16px 16px;
  }

  .pin-image-stage img {
    display: block;
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 4px;
    box-shadow: 0 16px 44px rgba(0, 0, 0, 0.32);
  }

  .app-shell {
    display: grid;
    grid-template-columns: 176px minmax(0, 1fr);
    width: 100%;
    height: 100vh;
    overflow: hidden;
    background: #f4f6f8;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px;
    background: #111827;
    color: #e5e7eb;
    border-right: 1px solid #0b1220;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-bottom: 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .brand-mark {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    color: #f8fafc;
    background: #2563eb;
    border-radius: 8px;
  }

  h1,
  h2,
  h3,
  p {
    margin: 0;
  }

  h1 {
    color: #ffffff;
    font-size: 1.05rem;
    font-weight: 700;
  }

  .brand p {
    margin-top: 2px;
    color: #94a3b8;
    font-size: 0.74rem;
  }

  .filter-nav {
    display: grid;
    gap: 6px;
  }

  .filter-button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 34px;
    padding: 8px 10px;
    color: #cbd5e1;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 7px;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
  }

  .filter-button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .filter-button:hover,
  .filter-button.active {
    color: #ffffff;
    background: #1f2937;
    border-color: #334155;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    flex: 0 0 auto;
    background: #22c55e;
    border-radius: 999px;
    box-shadow: 0 0 0 4px rgba(34, 197, 94, 0.14);
  }

  .workspace {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    padding: 14px 16px;
    gap: 10px;
    overflow: hidden;
  }

  .toolbar {
    display: grid;
    grid-template-columns: minmax(150px, 0.42fr) minmax(320px, 1fr);
    align-items: start;
    gap: 10px;
  }

  .toolbar-heading {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    margin-top: 1px;
  }

  .toolbar-tools {
    display: grid;
    min-width: 0;
    gap: 7px;
  }

  .toolbar-primary,
  .toolbar-secondary {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
  }

  .quick-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
  }

  .tool-button,
  .icon-tool {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 32px;
    color: #334155;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 7px;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
  }

  .tool-button {
    padding: 0 10px;
    white-space: nowrap;
  }

  .icon-tool {
    width: 34px;
    flex: 0 0 auto;
  }

  .tool-button:hover,
  .icon-tool:hover {
    color: #1d4ed8;
    background: #eff6ff;
    border-color: #bfdbfe;
  }

  .tool-button:disabled {
    cursor: wait;
    color: #64748b;
    background: #f8fafc;
  }

  .tool-button:disabled :global(svg) {
    animation: spin 900ms linear infinite;
  }

  .eyebrow {
    color: #64748b;
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  h2 {
    margin-top: 2px;
    color: #0f172a;
    font-size: 1.22rem;
    font-weight: 760;
  }

  .day-field {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 0 10px;
    width: 132px;
    flex: 0 0 auto;
    color: #475569;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 8px;
    font-size: 0.82rem;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
  }

  .day-field input {
    min-width: 0;
    width: 100%;
    color: #172033;
    background: transparent;
    border: 0;
    outline: 0;
  }

  .clear-date {
    display: grid;
    width: 24px;
    height: 24px;
    flex: 0 0 auto;
    place-items: center;
    color: #64748b;
    background: #f1f5f9;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
  }

  .date-shortcuts {
    display: flex;
    gap: 6px;
    min-width: 0;
    flex: 1 1 auto;
    overflow-x: auto;
    padding-bottom: 1px;
  }

  .date-shortcuts button {
    min-height: 28px;
    flex: 0 0 auto;
    padding: 0 8px;
    color: #475569;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 999px;
    cursor: pointer;
    font-size: 0.76rem;
  }

  .date-shortcuts button:hover,
  .date-shortcuts button.active {
    color: #1d4ed8;
    background: #eff6ff;
    border-color: #bfdbfe;
  }

  .search-field {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
    width: min(230px, 42vw);
    flex: 0 1 230px;
    padding: 0 10px;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 8px;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
  }

  .search-field :global(svg) {
    color: #64748b;
    flex: 0 0 auto;
  }

  .search-field input {
    width: 100%;
    min-width: 0;
    color: #172033;
    background: transparent;
    border: 0;
    outline: 0;
  }

  .search-field input::placeholder {
    color: #94a3b8;
  }

  .clear-search {
    display: grid;
    width: 26px;
    height: 26px;
    flex: 0 0 auto;
    place-items: center;
    color: #64748b;
    background: #f1f5f9;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
  }

  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 0.86rem;
  }

  .notice.error {
    color: #9f1239;
    background: #fff1f2;
    border: 1px solid #fecdd3;
  }

  .toast-stack {
    position: fixed;
    right: 14px;
    bottom: 14px;
    z-index: 1300;
    display: grid;
    gap: 8px;
    width: min(340px, calc(100% - 28px));
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 0.86rem;
    box-shadow: 0 18px 34px rgba(15, 23, 42, 0.14);
  }

  .toast.success {
    color: #166534;
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
  }

  .toast.error {
    color: #9f1239;
    background: #fff1f2;
    border: 1px solid #fecdd3;
  }

  .settings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgba(15, 23, 42, 0.28);
  }

  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    padding: 0;
    background: rgba(15, 23, 42, 0.34);
    border: 0;
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .confirm-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    z-index: 31;
    display: grid;
    gap: 14px;
    width: min(360px, calc(100vw - 32px));
    padding: 16px;
    color: #172033;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 12px;
    box-shadow: 0 24px 70px rgba(15, 23, 42, 0.22);
    transform: translate(-50%, -50%);
  }

  .confirm-dialog header {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .confirm-icon {
    display: grid;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    place-items: center;
    color: #b42338;
    background: #fff1f2;
    border: 1px solid #fecdd3;
    border-radius: 9px;
  }

  .confirm-dialog h2 {
    margin: 0;
    color: #172033;
    font-size: 1rem;
    line-height: 1.2;
  }

  .confirm-dialog p {
    margin: 4px 0 0;
    color: #64748b;
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .confirm-preview {
    max-height: 84px;
    padding: 10px 12px;
    overflow: hidden;
    color: #334155;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 9px;
    font-size: 0.86rem;
    line-height: 1.45;
    text-overflow: ellipsis;
  }

  .confirm-dialog footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .settings-panel {
    position: fixed;
    top: 0;
    right: 0;
    z-index: 21;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    width: min(620px, 96vw);
    height: 100vh;
    color: #172033;
    background: #ffffff;
    border-left: 1px solid #d9e0ea;
    box-shadow: -18px 0 44px rgba(15, 23, 42, 0.16);
  }

  .settings-header,
  .settings-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 14px;
    border-bottom: 1px solid #edf1f6;
  }

  .settings-header h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .settings-header button {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    color: #475569;
    background: #f8fafc;
    border: 1px solid #d9e0ea;
    border-radius: 7px;
    cursor: pointer;
  }

  .settings-workspace {
    display: grid;
    grid-template-columns: 136px minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
  }

  .settings-nav {
    display: grid;
    align-content: start;
    gap: 6px;
    min-width: 0;
    padding: 14px 10px;
    overflow: auto;
    background: #f8fafc;
    border-right: 1px solid #edf1f6;
  }

  .settings-nav button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 36px;
    padding: 0 10px;
    color: #64748b;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.84rem;
    font-weight: 600;
    text-align: left;
  }

  .settings-nav button:hover {
    color: #172033;
    background: #ffffff;
    border-color: #e2e8f0;
  }

  .settings-nav button.active {
    color: #0f766e;
    background: #ffffff;
    border-color: #cbdadd;
    box-shadow: 0 8px 18px rgba(15, 23, 42, 0.08);
  }

  .settings-content {
    display: grid;
    align-content: start;
    gap: 16px;
    min-height: 0;
    padding: 16px;
    overflow: auto;
  }

  .switch-row,
  .field-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    min-height: 42px;
    color: #172033;
    font-size: 0.88rem;
  }

  .switch-row span {
    grid-column: 1;
    grid-row: 1;
    min-width: 0;
  }

  .switch-row input[type='checkbox'] {
    position: relative;
    grid-column: 2;
    grid-row: 1;
    width: 38px;
    height: 22px;
    margin: 0;
    appearance: none;
    background: #d9e2e7;
    border: 1px solid #c4d0d8;
    border-radius: 999px;
    cursor: pointer;
    transition:
      background 160ms ease,
      border-color 160ms ease;
  }

  .switch-row input[type='checkbox']::after {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    content: '';
    background: #ffffff;
    border-radius: 50%;
    box-shadow: 0 1px 3px rgba(15, 23, 42, 0.2);
    transition: transform 160ms ease;
  }

  .switch-row input[type='checkbox']:checked {
    background: #0f766e;
    border-color: #0b5d56;
  }

  .switch-row input[type='checkbox']:checked::after {
    transform: translateX(16px);
  }

  .switch-row input[type='checkbox']:focus-visible {
    outline: 2px solid #99f6e4;
    outline-offset: 2px;
  }

  .field-row input,
  .field-row select {
    width: 118px;
    min-height: 32px;
    padding: 0 8px;
    color: #172033;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 7px;
  }

  .field-row select {
    width: min(220px, 58vw);
  }

  .field-row input[type="text"] {
    width: min(286px, 58vw);
  }

  .settings-section {
    display: grid;
    align-content: start;
    gap: 14px;
    padding-top: 0;
    border-top: 0;
  }

  .settings-section-title {
    display: grid;
    gap: 3px;
  }

  .settings-section-title h3 {
    margin: 0;
    color: #475569;
    font-size: 0.82rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .settings-section-title p {
    max-width: 34em;
    margin: 0;
    color: #64748b;
    font-size: 0.78rem;
    line-height: 1.45;
  }

  .inline-section-title {
    margin-top: 8px;
    padding-top: 14px;
    border-top: 1px solid #edf1f6;
  }

  .first-inline-section-title {
    margin-top: 0;
    padding-top: 0;
    border-top: 0;
  }

  .about-profile {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 16px 18px 18px;
    text-align: center;
    background: #fbfcfd;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
  }

  .about-avatar {
    display: block;
    width: 104px;
    height: 104px;
    object-fit: cover;
    object-position: center;
    image-rendering: auto;
    filter: contrast(0.94) saturate(0.92);
    background: #edf2f7;
    border: 1px solid #d9e0ea;
    border-radius: 22px;
    box-shadow: 0 8px 18px rgba(15, 23, 42, 0.08);
  }

  .about-profile-copy,
  .about-block,
  .about-link span {
    display: grid;
    min-width: 0;
  }

  .about-profile-copy {
    justify-items: center;
    gap: 3px;
    width: min(100%, 420px);
  }

  .about-eyebrow {
    color: #0f766e;
    font-size: 0.68rem;
    font-weight: 720;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .about-profile-copy h3,
  .about-block h4 {
    margin: 0;
    color: #172033;
  }

  .about-profile-copy h3 {
    font-size: 1.04rem;
    font-weight: 760;
  }

  .about-profile-copy p,
  .about-block p {
    margin: 0;
    color: #64748b;
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .about-profile-copy p {
    text-wrap: pretty;
  }

  .about-block {
    gap: 5px;
  }

  .about-block h4 {
    font-size: 0.8rem;
    font-weight: 700;
  }

  .about-list {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 8px 10px;
    background: rgba(248, 250, 252, 0.72);
    border: 1px solid #edf1f6;
    border-radius: 9px;
  }

  .about-list div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    min-height: 28px;
    color: #172033;
    font-size: 0.82rem;
  }

  .about-list dt {
    color: #64748b;
  }

  .about-list dd {
    margin: 0;
    min-width: 0;
    color: #172033;
    font-weight: 600;
    text-align: right;
  }

  .about-links {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .about-link {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 38px;
    min-width: 0;
    padding: 7px 9px;
    color: #0f766e;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 8px;
    text-decoration: none;
    transition:
      background 180ms ease,
      border-color 180ms ease,
      color 180ms ease;
  }

  .about-link:hover,
  .about-link:focus-visible {
    color: #0b5f59;
    background: #f0fdfa;
    border-color: #99f6e4;
  }

  .about-link :global(svg) {
    flex: 0 0 auto;
  }

  .about-link span {
    gap: 1px;
  }

  .about-link strong,
  .about-link small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .about-link strong {
    color: #172033;
    font-size: 0.82rem;
    font-weight: 680;
  }

  .about-link small {
    color: #64748b;
    font-size: 0.74rem;
  }

  .hotkey-hint,
  .cleanup-hint {
    color: #64748b;
    font-size: 0.78rem;
    line-height: 1.4;
    margin-top: -4px;
  }

  .cleanup-plan {
    padding: 8px 10px;
    color: #166534;
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
    border-radius: 7px;
    font-size: 0.8rem;
  }

  .cleanup-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .port-field {
    align-items: flex-start;
  }

  .port-field > label {
    padding-top: 8px;
  }

  .port-input-group {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    min-width: 0;
  }

  .compact-button {
    min-height: 32px;
    padding: 0 10px;
    white-space: nowrap;
  }

  .port-check-result {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 46px;
    padding: 9px 10px;
    color: #64748b;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    font-size: 0.8rem;
    line-height: 1.4;
  }

  .port-check-result p {
    margin: 0;
  }

  .port-check-result.available {
    color: #166534;
    background: #f0fdf4;
    border-color: #bbf7d0;
  }

  .port-check-result.occupied {
    color: #9f1239;
    background: #fff1f2;
    border-color: #fecdd3;
  }

  .port-hint {
    margin: -4px 0 0;
    color: #64748b;
    font-size: 0.78rem;
    line-height: 1.4;
  }

  .restart-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 12px;
    color: #172033;
    background: #ecfeff;
    border: 1px solid #a5f3fc;
    border-radius: 9px;
  }

  .restart-card div {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .restart-card strong {
    color: #164e63;
    font-size: 0.84rem;
  }

  .restart-card span {
    color: #64748b;
    font-size: 0.76rem;
  }

  .field-row input.recording {
    border-color: #007aff;
    background: rgba(0, 122, 255, 0.05);
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% {
      box-shadow: 0 0 0 0 rgba(0, 122, 255, 0.4);
    }
    50% {
      box-shadow: 0 0 0 4px rgba(0, 122, 255, 0);
    }
  }

  .settings-footer {
    justify-content: flex-end;
    border-top: 1px solid #edf1f6;
    border-bottom: 0;
  }

  @media (max-width: 480px) {
    .settings-panel {
      width: 100vw;
    }

    .settings-workspace {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr);
    }

    .settings-nav {
      display: flex;
      gap: 6px;
      padding: 10px 12px;
      overflow-x: auto;
      background: #f8fafc;
      border-right: 0;
      border-bottom: 1px solid #edf1f6;
    }

    .settings-nav button {
      width: auto;
      flex: 0 0 auto;
      padding: 0 10px;
    }

    .about-profile {
      justify-items: center;
    }

    .about-avatar {
      width: 92px;
      height: 92px;
    }

    .about-links {
      grid-template-columns: 1fr;
    }

    .restart-card {
      align-items: stretch;
      flex-direction: column;
      width: min(220px, 58vw);
    }
  }

  .ghost-button,
  .primary-button {
    min-height: 34px;
    padding: 0 12px;
    border-radius: 7px;
    cursor: pointer;
  }

  .ghost-button {
    color: #475569;
    background: #ffffff;
    border: 1px solid #d9e0ea;
  }

  .primary-button {
    color: #ffffff;
    background: #2563eb;
    border: 1px solid #1d4ed8;
  }

  .danger-button {
    min-height: 34px;
    padding: 0 12px;
    color: #ffffff;
    background: #b42338;
    border: 1px solid #97182c;
    border-radius: 7px;
    cursor: pointer;
  }

  .danger-button:disabled {
    cursor: wait;
    opacity: 0.72;
  }

  .primary-button:disabled {
    cursor: wait;
    opacity: 0.7;
  }

  .history-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    overflow: hidden;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 7px;
    box-shadow: 0 8px 20px rgba(15, 23, 42, 0.055);
  }

  .empty-state {
    display: grid;
    min-height: 360px;
    place-items: center;
    align-content: center;
    gap: 10px;
    padding: 34px;
    color: #64748b;
    text-align: center;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-state :global(svg) {
    color: #94a3b8;
  }

  .empty-state h3 {
    color: #172033;
    font-size: 1rem;
  }

  .empty-state p {
    color: #64748b;
    font-size: 0.88rem;
  }

  .items-list {
    display: grid;
    min-height: 0;
    max-height: none;
    flex: 1;
    overflow: auto;
  }

  .item {
    display: block;
    padding: 9px 12px;
    border-bottom: 1px solid #edf1f6;
    background: #ffffff;
  }

  .item:hover {
    background: #f8fafc;
  }

  .item.pinned {
    background: #fffbeb;
  }

  .item-main {
    min-width: 0;
  }

  .item-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
  }

  .item-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
    color: #64748b;
    font-size: 0.76rem;
  }

  .type-pill,
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-height: 20px;
    padding: 1px 6px;
    background: #eef2ff;
    border-radius: 999px;
    color: #3730a3;
    font-weight: 650;
  }

  .badge {
    color: #155e75;
    background: #ecfeff;
  }

  .text-content {
    display: block;
    width: 100%;
    margin-top: 7px;
    max-height: 4.9em;
    overflow: hidden;
    color: #172033;
    font: inherit;
    font-size: 0.92rem;
    line-height: 1.45;
    text-align: left;
    word-break: break-word;
    background: #f8fafc;
    border: 1px solid transparent;
    padding: 6px 7px;
    border-radius: 4px;
    transition: background 0.18s ease, border-color 0.18s ease, transform 0.18s ease;
  }

  .text-content.copyable:hover,
  .text-content.copyable:focus-visible {
    background: #f1f5f9;
    border-color: #cbd5e1;
  }

  .text-content.copyable:focus-visible {
    outline: 2px solid #5eead4;
    outline-offset: 2px;
  }

  .text-content.copyable:active {
    transform: translateY(1px);
  }

  .annotation-note {
    display: grid;
    gap: 4px;
    margin-top: 9px;
    padding: 8px 10px;
    color: #334155;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
  }

  .annotation-note span {
    color: #64748b;
    font-size: 0.74rem;
    font-weight: 700;
  }

  .annotation-note p {
    margin: 0;
    font-size: 0.88rem;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .annotation-editor {
    margin-top: 9px;
  }

  .edit-area {
    margin-top: 9px;
  }

  .edit-area textarea,
  .annotation-editor textarea {
    width: 100%;
    min-height: 88px;
    padding: 10px;
    color: #172033;
    font-size: 0.92rem;
    line-height: 1.45;
    background: #ffffff;
    border: 2px solid #3b82f6;
    border-radius: 8px;
    outline: none;
    resize: vertical;
    font-family: inherit;
  }

  .edit-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .edit-actions button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 0.88rem;
    border: 1px solid #d9e0ea;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-save {
    color: #ffffff;
    background: #3b82f6;
    border-color: #3b82f6;
  }

  .btn-save:hover {
    background: #2563eb;
  }

  .btn-cancel {
    color: #64748b;
    background: #ffffff;
  }

  .btn-cancel:hover {
    background: #f1f5f9;
  }

  .image-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    min-width: 0;
    color: #475569;
    font-size: 0.82rem;
  }

  .image-summary strong {
    color: #172033;
    font-size: 0.9rem;
    font-weight: 700;
    white-space: nowrap;
  }

  .image-summary span {
    min-width: 0;
    overflow: hidden;
    color: #64748b;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .image-preview {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: fit-content;
    max-width: 100%;
    margin-top: 9px;
    max-height: 164px;
    overflow: hidden;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .image-preview:hover {
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .image-preview img {
    display: block;
    width: auto;
    max-width: 100%;
    max-height: 164px;
    object-fit: contain;
  }

  .image-viewer-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.85);
    backdrop-filter: blur(4px);
  }

  .image-viewer-content {
    position: relative;
    max-width: 90vw;
    max-height: 90vh;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .image-viewer-content img {
    max-width: 100%;
    max-height: 90vh;
    object-fit: contain;
    border-radius: 8px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }

  .image-viewer-close {
    position: absolute;
    top: -50px;
    right: 0;
    display: grid;
    width: 40px;
    height: 40px;
    place-items: center;
    color: #ffffff;
    background: rgba(0, 0, 0, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .image-viewer-close:hover {
    background: rgba(0, 0, 0, 0.7);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .image-loading {
    margin-top: 10px;
    padding: 18px;
    color: #64748b;
    background: #f8fafc;
    border: 1px dashed #cbd5e1;
    border-radius: 8px;
  }

  .item-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .item-actions button {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    color: #64748b;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 7px;
    cursor: pointer;
  }

  .item-actions .primary-action,
  .item-actions button.active {
    color: #334155;
    background: #ffffff;
    border-color: #d9e0ea;
  }

  .item-actions .secondary-action {
    opacity: 0.54;
  }

  .item:hover .item-actions .secondary-action,
  .item:focus-within .item-actions .secondary-action,
  .item-actions .secondary-action.active {
    opacity: 1;
  }

  .item-actions button:hover,
  .item-actions button.active {
    color: #1d4ed8;
    background: #eff6ff;
    border-color: #bfdbfe;
  }

  .item-actions .danger-action:hover,
  .item-actions .danger-action:focus-visible {
    color: #b42338;
    background: #fff1f2;
    border-color: #fecdd3;
  }

  .context-menu {
    position: fixed;
    z-index: 1200;
    display: grid;
    min-width: 168px;
    padding: 6px;
    background: #ffffff;
    border: 1px solid #dbe3ee;
    border-radius: 10px;
    box-shadow:
      0 18px 44px rgba(15, 23, 42, 0.18),
      0 1px 0 rgba(255, 255, 255, 0.9) inset;
  }

  .context-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    color: #253347;
    font: inherit;
    font-size: 0.86rem;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 7px;
    cursor: pointer;
  }

  .context-menu button:hover {
    color: #0f172a;
    background: #f1f5f9;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  @media (max-width: 720px) {
    .app-shell {
      display: flex;
      flex-direction: column;
    }

    .sidebar {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr);
      grid-template-areas:
        'brand filters';
      align-items: center;
      gap: 8px;
      flex: 0 0 auto;
      padding: 10px 14px;
    }

    .brand {
      grid-area: brand;
      min-width: 0;
      border-bottom: 0;
      padding-bottom: 0;
    }

    .filter-nav {
      grid-area: filters;
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      min-width: 0;
      gap: 8px;
    }

    .filter-button {
      justify-content: center;
      width: auto;
      min-width: 0;
      min-height: 30px;
      padding: 6px 8px;
      text-align: center;
    }

    .workspace {
      flex: 1;
      padding: 12px 16px 14px;
    }

    .toolbar {
      grid-template-columns: 1fr;
      gap: 8px;
    }

    .quick-actions {
      justify-content: stretch;
    }

    .tool-button {
      flex: 1 1 0;
      min-width: 0;
      padding: 0 8px;
    }

    h2 {
      font-size: 1.22rem;
    }

    .search-field {
      min-height: 34px;
    }

    .item {
      padding: 10px 12px;
    }

    .item-row {
      gap: 8px;
    }

    .item-actions {
      justify-content: flex-end;
    }
  }

  /* Redesign layer: compact desktop-tool visual system */
  :global(*) {
    box-sizing: border-box;
  }

  :global(html),
  :global(body) {
    width: 100%;
    height: 100%;
  }

  :global(body) {
    --accent: #0f766e;
    --accent-strong: #0b5d56;
    --accent-soft: #e1f4f1;
    --accent-line: #9bd1ca;
    --ink: #172124;
    background:
      repeating-linear-gradient(
        0deg,
        rgba(39, 58, 61, 0.026) 0,
        rgba(39, 58, 61, 0.026) 1px,
        transparent 1px,
        transparent 5px
      ),
      #eef2f3;
    color: #172124;
    font-family:
      Aptos,
      'Segoe UI Variable',
      'Segoe UI',
      ui-sans-serif,
      system-ui,
      -apple-system,
      BlinkMacSystemFont,
      sans-serif;
    font-variant-numeric: tabular-nums;
  }

  :global(button),
  :global(input),
  :global(textarea) {
    font: inherit;
  }

  :global(button) {
    transition:
      background 180ms ease,
      border-color 180ms ease,
      color 180ms ease,
      box-shadow 180ms ease,
      transform 140ms ease,
      opacity 180ms ease;
  }

  :global(button:not(:disabled):active) {
    transform: translateY(1px) scale(0.985);
  }

  :global(button:focus-visible),
  :global(input:focus-visible),
  :global(textarea:focus-visible),
  .image-preview:focus-visible {
    outline: 2px solid rgba(15, 118, 110, 0.38);
    outline-offset: 2px;
  }

  .app-shell {
    --ink: #172124;
    --muted: #66767a;
    --soft: #8b9a9e;
    --line: #d8e1e4;
    --line-soft: #e8eef0;
    --paper: #fbfcfc;
    --surface: #ffffff;
    --surface-2: #f4f7f7;
    --accent: #0f766e;
    --accent-strong: #0b5d56;
    --accent-soft: #e1f4f1;
    --accent-line: #9bd1ca;
    --danger: #b42338;
    --success: #177245;
    display: grid;
    grid-template-columns: 168px minmax(0, 1fr);
    height: 100vh;
    height: 100dvh;
    background:
      linear-gradient(90deg, #101819 0 168px, transparent 168px),
      linear-gradient(180deg, #f9fbfb 0%, #eef2f3 100%);
  }

  .sidebar {
    gap: 12px;
    padding: 12px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.04), transparent 46%),
      #101819;
    color: #e8eeee;
    border-right: 1px solid rgba(221, 232, 234, 0.12);
  }

  .brand {
    gap: 9px;
    padding: 2px 2px 12px;
    border-bottom: 1px solid rgba(232, 238, 238, 0.11);
  }

  .brand-mark {
    width: 34px;
    height: 34px;
    color: #eafffb;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.16), transparent),
      #0f766e;
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 10px;
    box-shadow:
      0 10px 28px rgba(6, 78, 72, 0.34),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
  }

  h1 {
    color: #f8fbfb;
    font-size: 1.02rem;
    font-weight: 760;
    letter-spacing: 0;
    line-height: 1.05;
  }

  .brand p {
    color: #a8b8b7;
    font-size: 0.72rem;
  }

  .filter-nav {
    gap: 5px;
  }

  .filter-button {
    min-height: 34px;
    padding: 8px 9px;
    color: #becac9;
    border-radius: 9px;
    border-color: transparent;
  }

  .filter-button:hover {
    color: #f6fbfb;
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.08);
  }

  .filter-button.active {
    color: #f8fffe;
    background:
      linear-gradient(90deg, rgba(15, 118, 110, 0.34), rgba(255, 255, 255, 0.06)),
      rgba(255, 255, 255, 0.06);
    border-color: rgba(155, 209, 202, 0.34);
    box-shadow: inset 3px 0 0 #54d0c4;
  }

  .status-dot {
    width: 7px;
    height: 7px;
    background: #36c687;
    box-shadow: 0 0 0 5px rgba(54, 198, 135, 0.13);
  }

  .workspace {
    position: relative;
    gap: 10px;
    padding: 13px 14px 14px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.72), rgba(246, 249, 249, 0.8)),
      var(--paper);
  }

  .workspace::before {
    position: absolute;
    inset: 0;
    pointer-events: none;
    content: '';
    background:
      linear-gradient(90deg, rgba(15, 118, 110, 0.045), transparent 28%),
      repeating-linear-gradient(
        90deg,
        rgba(23, 33, 36, 0.025) 0,
        rgba(23, 33, 36, 0.025) 1px,
        transparent 1px,
        transparent 24px
      );
    mask-image: linear-gradient(180deg, #000, transparent 58%);
  }

  .toolbar,
  .history-panel,
  .notice {
    position: relative;
    z-index: 1;
  }

  .toolbar {
    grid-template-columns: minmax(156px, 0.38fr) minmax(330px, 1fr);
    align-items: start;
    gap: 10px;
  }

  .toolbar-title {
    min-width: 0;
    padding-top: 0;
  }

  .toolbar-heading {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    margin-top: 1px;
  }

  .eyebrow {
    color: #789093;
    font-size: 0.62rem;
    font-weight: 760;
    letter-spacing: 0.07em;
  }

  h2 {
    margin-top: 1px;
    color: var(--ink);
    font-size: clamp(1.14rem, 3.2vw, 1.34rem);
    font-weight: 780;
    line-height: 1.08;
    text-wrap: balance;
  }

  .toolbar-context {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 25px;
    margin: 0;
    padding: 0 9px;
    color: #3f575b;
    background: rgba(255, 255, 255, 0.74);
    border: 1px solid var(--line-soft);
    border-radius: 999px;
    font-size: 0.72rem;
    line-height: 1;
    white-space: nowrap;
  }

  .toolbar-tools {
    gap: 6px;
  }

  .toolbar-primary,
  .toolbar-secondary {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
  }

  .toolbar-secondary {
    justify-content: stretch;
  }

  .quick-actions {
    justify-content: flex-end;
    gap: 6px;
  }

  .tool-button,
  .icon-tool,
  .day-field,
  .search-field,
  .date-shortcuts button,
  .item-actions button,
  .ghost-button,
  .primary-button,
  .settings-header button,
  .edit-actions button {
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.8) inset,
      0 8px 20px rgba(34, 58, 63, 0.055);
  }

  .tool-button,
  .icon-tool {
    min-height: 30px;
    color: #2d4547;
    background: rgba(255, 255, 255, 0.86);
    border-color: rgba(188, 204, 208, 0.88);
    border-radius: 9px;
  }

  .tool-button:hover,
  .icon-tool:hover {
    color: var(--accent-strong);
    background: #f3fbfa;
    border-color: var(--accent-line);
  }

  .tool-button:disabled {
    color: #77888b;
    background: #eef3f3;
    border-color: #d9e1e3;
  }

  .day-field,
  .search-field {
    background: rgba(255, 255, 255, 0.88);
    border-color: rgba(193, 207, 211, 0.92);
    border-radius: 10px;
  }

  .day-field {
    width: 130px;
    min-height: 32px;
    flex: 0 0 130px;
    color: var(--muted);
  }

  .calendar-field {
    position: relative;
    overflow: visible;
  }

  .calendar-field :global(svg) {
    flex: 0 0 auto;
    color: #718184;
  }

  .day-field input,
  .search-field input {
    color: var(--ink);
  }

  .day-field input {
    cursor: pointer;
  }

  .day-field:focus-within,
  .search-field:focus-within {
    border-color: var(--accent-line);
    box-shadow:
      0 0 0 3px rgba(15, 118, 110, 0.1),
      0 8px 20px rgba(34, 58, 63, 0.055);
  }

  .clear-date,
  .clear-search {
    color: #5b6d70;
    background: #edf4f3;
    border-radius: 8px;
  }

  .clear-date:hover,
  .clear-search:hover {
    color: var(--accent-strong);
    background: var(--accent-soft);
  }

  .date-shortcuts {
    gap: 5px;
    flex: 1 1 auto;
    scrollbar-width: thin;
  }

  .date-shortcuts button {
    min-height: 26px;
    color: #506468;
    background: rgba(255, 255, 255, 0.72);
    border-color: rgba(193, 207, 211, 0.82);
    border-radius: 8px;
    font-size: 0.74rem;
    font-weight: 620;
  }

  .date-shortcuts button:hover,
  .date-shortcuts button.active {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-color: var(--accent-line);
  }

  :global(.clipmaster-date-picker.flatpickr-calendar) {
    width: 292px;
    padding: 10px;
    color: var(--ink);
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(246, 250, 250, 0.98)),
      #ffffff;
    border: 1px solid #c9d7da;
    border-radius: 14px;
    box-shadow:
      0 22px 60px rgba(23, 42, 47, 0.18),
      0 1px 0 rgba(255, 255, 255, 0.95) inset;
    font-family: inherit;
    overflow: hidden;
  }

  :global(.clipmaster-date-picker.flatpickr-calendar::before),
  :global(.clipmaster-date-picker.flatpickr-calendar::after) {
    display: none;
  }

  :global(.clipmaster-date-picker .flatpickr-months) {
    align-items: center;
    margin-bottom: 6px;
  }

  :global(.clipmaster-date-picker .flatpickr-month) {
    height: 34px;
    color: var(--ink);
  }

  :global(.clipmaster-date-picker .flatpickr-current-month) {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 34px;
    left: 50px;
    width: calc(100% - 100px);
    padding: 0;
    font-size: 0.92rem;
    font-weight: 760;
  }

  :global(.clipmaster-date-picker .flatpickr-current-month .numInputWrapper) {
    width: 64px;
  }

  :global(.clipmaster-date-picker .flatpickr-current-month input.cur-year) {
    color: var(--ink);
    font-size: 0.92rem;
    font-weight: 760;
  }

  :global(.clipmaster-date-picker .flatpickr-current-month .flatpickr-monthDropdown-months) {
    height: 30px;
    color: var(--ink);
    background: transparent;
    border-radius: 8px;
    font-size: 0.92rem;
    font-weight: 760;
  }

  :global(.clipmaster-date-picker .flatpickr-prev-month),
  :global(.clipmaster-date-picker .flatpickr-next-month) {
    display: grid;
    width: 42px;
    height: 30px;
    place-items: center;
    color: #51686c;
    border-radius: 9px;
    top: 10px;
    padding: 0;
  }

  .search-field {
    width: min(220px, 38vw);
    min-height: 32px;
    flex: 0 1 220px;
  }

  :global(.clipmaster-date-picker .date-nav-label) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 30px;
    font-size: 0.72rem;
    font-weight: 760;
    line-height: 1;
    letter-spacing: 0;
  }

  :global(.clipmaster-date-picker .flatpickr-prev-month:hover),
  :global(.clipmaster-date-picker .flatpickr-next-month:hover) {
    color: var(--accent-strong);
    background: var(--accent-soft);
  }

  :global(.clipmaster-date-picker .flatpickr-weekdays) {
    height: 28px;
    background: transparent;
  }

  :global(.clipmaster-date-picker span.flatpickr-weekday) {
    color: #66767a;
    font-size: 0.72rem;
    font-weight: 760;
  }

  :global(.clipmaster-date-picker .dayContainer) {
    gap: 3px;
    width: 270px;
    min-width: 270px;
    max-width: 270px;
  }

  :global(.clipmaster-date-picker .flatpickr-days) {
    width: 270px;
  }

  :global(.clipmaster-date-picker .flatpickr-day) {
    position: relative;
    max-width: 36px;
    height: 34px;
    line-height: 34px;
    color: #263b3e;
    border: 0;
    border-radius: 9px;
    font-size: 0.82rem;
  }

  :global(.clipmaster-date-picker .flatpickr-day:hover),
  :global(.clipmaster-date-picker .flatpickr-day:focus) {
    color: var(--accent-strong);
    background: #edf8f6;
  }

  :global(.clipmaster-date-picker .flatpickr-day.today) {
    color: var(--accent-strong);
    background: rgba(15, 118, 110, 0.08);
    box-shadow: inset 0 0 0 1px rgba(15, 118, 110, 0.24);
  }

  :global(.clipmaster-date-picker .flatpickr-day.selected),
  :global(.clipmaster-date-picker .flatpickr-day.selected:hover) {
    color: #ffffff;
    background: var(--accent);
    box-shadow: 0 8px 18px rgba(15, 118, 110, 0.24);
  }

  :global(.clipmaster-date-picker .flatpickr-day.prevMonthDay),
  :global(.clipmaster-date-picker .flatpickr-day.nextMonthDay) {
    color: #b5c1c4;
  }

  :global(.clipmaster-date-picker .flatpickr-day.has-clipboard-items::after) {
    position: absolute;
    left: 50%;
    bottom: 4px;
    width: 4px;
    height: 4px;
    content: '';
    background: var(--accent);
    border-radius: 999px;
    transform: translateX(-50%);
  }

  :global(.clipmaster-date-picker .flatpickr-day.selected.has-clipboard-items::after) {
    background: #ffffff;
  }

  .search-field {
    min-height: 32px;
  }

  .search-field :global(svg) {
    color: #718184;
  }

  .search-field input::placeholder {
    color: #91a0a3;
  }

  .notice {
    min-height: 34px;
    border-radius: 10px;
    font-size: 0.82rem;
    box-shadow: 0 8px 22px rgba(34, 58, 63, 0.06);
  }

  .notice.error {
    color: var(--danger);
    background: #fff4f5;
    border-color: #f2bdc5;
  }

  .toast-stack {
    right: 16px;
    bottom: 16px;
  }

  .toast {
    border-radius: 11px;
    font-size: 0.82rem;
    box-shadow:
      0 16px 32px rgba(34, 58, 63, 0.16),
      0 1px 0 rgba(255, 255, 255, 0.85) inset;
  }

  .toast.success {
    color: var(--success);
    background: #f0faf4;
    border-color: #b8e5c9;
  }

  .toast.error {
    color: var(--danger);
    background: #fff4f5;
    border-color: #f2bdc5;
  }

  .history-panel {
    overflow: hidden;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.9), rgba(250, 252, 252, 0.94)),
      #ffffff;
    border: 1px solid rgba(193, 207, 211, 0.9);
    border-radius: 12px;
    box-shadow:
      0 18px 46px rgba(41, 61, 66, 0.08),
      0 1px 0 rgba(255, 255, 255, 0.9) inset;
  }

  .loading-stack {
    display: grid;
    gap: 10px;
    padding: 14px;
  }

  .loading-head {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.82rem;
    font-weight: 650;
  }

  .loading-head :global(svg) {
    animation: spin 900ms linear infinite;
  }

  .skeleton-item {
    display: grid;
    gap: 8px;
    padding: 13px;
    background: #f6f9f9;
    border: 1px solid var(--line-soft);
    border-radius: 10px;
  }

  .skeleton-meta,
  .skeleton-line {
    display: block;
    height: 10px;
    overflow: hidden;
    background:
      linear-gradient(90deg, #e5eded 0%, #f8fbfb 50%, #e5eded 100%);
    background-size: 220% 100%;
    border-radius: 999px;
    animation: shimmer 1.4s ease-in-out infinite;
  }

  .skeleton-meta {
    width: 34%;
    height: 12px;
  }

  .skeleton-line {
    width: 58%;
  }

  .skeleton-line.wide {
    width: 86%;
  }

  @keyframes shimmer {
    0% {
      background-position: 120% 0;
    }
    100% {
      background-position: -120% 0;
    }
  }

  .empty-state {
    min-height: 340px;
    gap: 8px;
    color: var(--muted);
  }

  .empty-mark {
    display: grid;
    width: 68px;
    height: 68px;
    margin-bottom: 4px;
    place-items: center;
    color: var(--accent);
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.9), rgba(225, 244, 241, 0.8)),
      var(--accent-soft);
    border: 1px solid #c8e5e0;
    border-radius: 18px;
    box-shadow: 0 18px 34px rgba(15, 118, 110, 0.12);
  }

  .empty-state h3 {
    color: var(--ink);
    font-size: 1rem;
    font-weight: 720;
  }

  .empty-state p {
    max-width: 34ch;
    color: var(--muted);
    font-size: 0.86rem;
    line-height: 1.5;
    text-wrap: pretty;
  }

  .items-list {
    scrollbar-width: thin;
    scrollbar-color: #b9c7ca transparent;
  }

  .item {
    position: relative;
    padding: 9px 12px;
    background: rgba(255, 255, 255, 0.72);
    border-bottom-color: var(--line-soft);
    transition:
      background 180ms ease,
      box-shadow 180ms ease;
  }

  .item::before {
    position: absolute;
    inset: 10px auto 10px 0;
    width: 3px;
    content: '';
    background: transparent;
    border-radius: 0 4px 4px 0;
  }

  .item:hover {
    background: #f7fbfa;
    box-shadow: inset 0 0 0 1px rgba(15, 118, 110, 0.08);
  }

  .item.pinned {
    background: #f5fbf4;
  }

  .item.pinned::before {
    background: #5fb76b;
  }

  .item-row {
    gap: 7px;
  }

  .item-meta {
    gap: 6px;
    color: var(--muted);
    font-size: 0.74rem;
  }

  .type-pill,
  .badge {
    min-height: 20px;
    padding: 1px 6px;
    color: #194a48;
    background: #e7f3f1;
    border: 1px solid #c9dfdb;
    border-radius: 7px;
    font-weight: 690;
  }

  .badge {
    color: #486327;
    background: #f0f6e8;
    border-color: #d8e8be;
  }

  .text-content {
    margin-top: 6px;
    padding: 6px 8px;
    max-height: 4.7em;
    color: var(--ink);
    background: #f7fbfa;
    border: 1px solid #dce8e6;
    border-radius: 8px;
    font-size: 0.91rem;
    line-height: 1.42;
    text-wrap: pretty;
  }

  .text-content.copyable:hover,
  .text-content.copyable:focus-visible {
    background: #eef6f4;
    border-color: #b9d8d2;
  }

  .annotation-note {
    background: #f0f6e8;
    border-color: #d8e8be;
  }

  .annotation-note span {
    color: #486327;
  }

  .annotation-note p {
    color: var(--ink);
  }

  .annotation-editor textarea {
    min-height: 86px;
    color: var(--ink);
    background: #fbfdfd;
    border: 1px solid var(--accent-line);
    border-radius: 10px;
    box-shadow: 0 0 0 3px rgba(15, 118, 110, 0.1);
  }

  .annotation-editor textarea:focus {
    background: #eef6f4;
    outline: none;
  }

  .edit-area textarea {
    min-height: 104px;
    color: var(--ink);
    background: #fbfdfd;
    border: 1px solid var(--accent-line);
    border-radius: 10px;
    box-shadow: 0 0 0 3px rgba(15, 118, 110, 0.1);
  }

  .edit-actions button,
  .item-actions button {
    border-radius: 8px;
  }

  .btn-save {
    background: var(--accent);
    border-color: var(--accent);
  }

  .btn-save:hover {
    background: var(--accent-strong);
  }

  .btn-cancel:hover {
    background: #eef4f4;
  }

  .image-summary {
    gap: 8px;
    color: var(--muted);
  }

  .image-summary strong {
    color: var(--ink);
    font-weight: 720;
  }

  .image-preview,
  .image-loading {
    background:
      repeating-linear-gradient(
        45deg,
        rgba(15, 118, 110, 0.04) 0,
        rgba(15, 118, 110, 0.04) 8px,
        transparent 8px,
        transparent 16px
      ),
      #f5f8f8;
    border-color: #d7e3e4;
    border-radius: 10px;
  }

  .image-preview:hover {
    border-color: var(--accent-line);
    box-shadow: 0 0 0 3px rgba(15, 118, 110, 0.1);
  }

  .image-viewer-overlay {
    z-index: 40;
    background: rgba(7, 16, 18, 0.84);
    backdrop-filter: blur(7px);
  }

  .image-viewer-content img {
    border-radius: 12px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.5);
  }

  .image-viewer-close {
    border-radius: 10px;
  }

  .item-actions {
    gap: 4px;
  }

  .item-actions button {
    width: 28px;
    height: 28px;
    color: #63777a;
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }

  .item-actions .primary-action,
  .item-actions button.active {
    color: #2d4547;
    background: rgba(255, 255, 255, 0.84);
    border-color: #d7e3e4;
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.84) inset,
      0 6px 14px rgba(34, 58, 63, 0.045);
  }

  .item-actions .secondary-action {
    opacity: 0.48;
  }

  .item:hover .item-actions .secondary-action,
  .item:focus-within .item-actions .secondary-action,
  .item-actions .secondary-action.active {
    opacity: 1;
  }

  .item-actions button:hover,
  .item-actions button.active {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-color: var(--accent-line);
  }

  .context-menu {
    background: #fbfdfd;
    border-color: #cbdcda;
    border-radius: 10px;
    box-shadow:
      0 18px 44px rgba(9, 24, 27, 0.18),
      0 1px 0 rgba(255, 255, 255, 0.9) inset;
  }

  .context-menu button {
    color: var(--ink);
    border-radius: 8px;
  }

  .context-menu button:hover {
    color: var(--accent-strong);
    background: var(--accent-soft);
  }

  .settings-backdrop {
    z-index: 20;
    background: rgba(9, 24, 27, 0.34);
    backdrop-filter: blur(2px);
  }

  .confirm-backdrop {
    background: rgba(9, 24, 27, 0.36);
  }

  .confirm-dialog {
    color: var(--ink);
    background:
      linear-gradient(180deg, #fbfcfc, #f5f8f8),
      #ffffff;
    border-color: #cbdadd;
    border-radius: 12px;
    box-shadow: 0 24px 70px rgba(25, 44, 49, 0.2);
  }

  .confirm-dialog h2 {
    color: var(--ink);
    font-size: 1.04rem;
    font-weight: 760;
  }

  .confirm-dialog p {
    color: var(--muted);
  }

  .confirm-preview {
    color: #2d4547;
    background: rgba(255, 255, 255, 0.72);
    border-color: var(--line-soft);
  }

  .settings-panel {
    z-index: 21;
    width: min(620px, 96vw);
    color: var(--ink);
    background:
      linear-gradient(180deg, #fbfcfc, #f3f7f7),
      #ffffff;
    border-left-color: #cbdadd;
    box-shadow: -24px 0 60px rgba(25, 44, 49, 0.18);
  }

  .settings-header,
  .settings-footer {
    padding: 13px 14px;
    border-color: var(--line-soft);
  }

  .settings-header h2 {
    color: var(--ink);
    font-size: 1.12rem;
    font-weight: 760;
  }

  .settings-header button {
    color: #4d6265;
    background: #f4f8f8;
    border-color: #d5e0e2;
  }

  .settings-header button:hover {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-color: var(--accent-line);
  }

  .settings-workspace {
    background: rgba(255, 255, 255, 0.46);
  }

  .settings-nav {
    background:
      linear-gradient(180deg, rgba(245, 249, 249, 0.96), rgba(238, 245, 245, 0.9)),
      #f5f8f8;
    border-right-color: var(--line-soft);
    scrollbar-width: thin;
  }

  .settings-nav button {
    color: var(--muted);
    border-radius: 9px;
    font-weight: 680;
  }

  .settings-nav button:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.82);
    border-color: #d6e2e4;
  }

  .settings-nav button.active {
    color: var(--accent-strong);
    background: #ffffff;
    border-color: var(--accent-line);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.9) inset,
      0 10px 22px rgba(25, 44, 49, 0.1);
  }

  .settings-content {
    gap: 16px;
    padding: 16px;
    scrollbar-width: thin;
  }

  .switch-row,
  .field-row {
    min-height: 40px;
    color: var(--ink);
    font-size: 0.86rem;
  }

  .switch-row input {
    width: 38px;
    height: 22px;
    background: #d7e1e3;
    border-color: #c6d4d7;
  }

  .switch-row input[type='checkbox']:checked {
    background: var(--accent);
    border-color: var(--accent-strong);
  }

  .field-row input,
  .field-row select {
    width: 118px;
    min-height: 32px;
    color: var(--ink);
    background: #ffffff;
    border-color: #cfdcdf;
    border-radius: 8px;
  }

  .field-row input:focus-visible,
  .field-row select:focus-visible {
    border-color: var(--accent-line);
  }

  .field-row select {
    width: min(220px, 58vw);
  }

  .field-row input[type='text'] {
    width: min(286px, 58vw);
  }

  .settings-section {
    gap: 12px;
  }

  .settings-section-title {
    gap: 4px;
  }

  .settings-section-title h3 {
    color: #496064;
    font-size: 0.75rem;
    font-weight: 760;
    letter-spacing: 0.08em;
  }

  .settings-section-title p {
    color: var(--muted);
    font-size: 0.78rem;
  }

  .inline-section-title {
    border-top-color: var(--line-soft);
  }

  .about-profile {
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.9), rgba(247, 250, 250, 0.82)),
      #f8fbfb;
    border-color: var(--line-soft);
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.88) inset;
  }

  .about-avatar {
    border-color: #cbdadd;
    box-shadow: 0 10px 22px rgba(25, 44, 49, 0.1);
  }

  .about-eyebrow {
    color: var(--accent-strong);
  }

  .about-profile-copy h3,
  .about-block h4,
  .about-link strong {
    color: var(--ink);
  }

  .about-profile-copy p,
  .about-block p,
  .about-link small {
    color: var(--muted);
  }

  .about-list {
    background: rgba(255, 255, 255, 0.58);
    border-color: var(--line-soft);
  }

  .about-list div {
    color: var(--ink);
  }

  .about-list dt {
    color: var(--muted);
  }

  .about-list dd {
    color: var(--ink);
    font-weight: 680;
  }

  .about-link {
    color: var(--accent-strong);
    background: rgba(255, 255, 255, 0.76);
    border-color: #cfdcdf;
  }

  .about-link:hover,
  .about-link:focus-visible {
    color: var(--accent);
    background: var(--accent-soft);
    border-color: var(--accent-line);
  }

  .item-actions .danger-action:hover,
  .item-actions .danger-action:focus-visible {
    color: var(--danger);
    background: #fff1f2;
    border-color: #fecdd3;
  }

  .hotkey-hint,
  .cleanup-hint {
    color: var(--muted);
  }

  .cleanup-plan {
    color: var(--success);
    background: #eefaf2;
    border-color: #bee4ca;
    border-radius: 8px;
  }

  .port-check-result {
    color: var(--muted);
    background: rgba(255, 255, 255, 0.58);
    border-color: var(--line-soft);
    border-radius: 8px;
  }

  .port-check-result.available {
    color: var(--success);
    background: #eefaf2;
    border-color: #bee4ca;
  }

  .port-check-result.occupied {
    color: var(--danger);
    background: #fff1f2;
    border-color: #fecdd3;
  }

  .port-hint {
    color: var(--muted);
  }

  .restart-card {
    color: var(--ink);
    background: rgba(238, 250, 252, 0.86);
    border-color: #bae6ed;
  }

  .restart-card strong {
    color: #155e75;
  }

  .restart-card span {
    color: var(--muted);
  }

  .ghost-button,
  .primary-button {
    min-height: 34px;
    border-radius: 8px;
    font-weight: 650;
  }

  .ghost-button {
    color: #41575a;
    background: rgba(255, 255, 255, 0.86);
    border-color: #cfdcdf;
  }

  .ghost-button:hover {
    color: var(--accent-strong);
    background: var(--accent-soft);
    border-color: var(--accent-line);
  }

  .primary-button {
    background: var(--accent);
    border-color: var(--accent-strong);
  }

  .primary-button:hover {
    background: var(--accent-strong);
  }

  .danger-button {
    min-height: 34px;
    padding: 0 12px;
    color: #ffffff;
    background: var(--danger);
    border: 1px solid #97182c;
    border-radius: 8px;
    font-weight: 650;
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.14) inset,
      0 10px 24px rgba(180, 35, 56, 0.18);
  }

  .danger-button:hover {
    background: #97182c;
  }

  .pin-toolbar {
    background: rgba(12, 20, 22, 0.92);
  }

  @media (max-width: 720px) {
    .settings-backdrop {
      display: none;
    }

    .settings-panel {
      width: 100vw;
      border-left: 0;
      box-shadow: none;
    }

    .app-shell {
      display: flex;
      flex-direction: column;
      background: linear-gradient(180deg, #101819 0 78px, #f7fafa 78px, #eef2f3 100%);
    }

    .sidebar {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr);
      grid-template-areas:
        'brand filters';
      align-items: center;
      gap: 8px;
      flex: 0 0 auto;
      padding: 10px 12px 9px;
      border-right: 0;
      border-bottom: 1px solid rgba(221, 232, 234, 0.12);
      box-shadow: 0 12px 34px rgba(7, 24, 29, 0.18);
    }

    .brand {
      min-width: 0;
      gap: 8px;
      padding: 0;
      border-bottom: 0;
    }

    .brand-mark {
      width: 32px;
      height: 32px;
      border-radius: 9px;
    }

    .filter-nav {
      grid-area: filters;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 6px;
      padding-top: 1px;
    }

    .filter-button {
      min-height: 31px;
      padding: 6px 7px;
      border-radius: 8px;
      font-size: 0.82rem;
    }

    .filter-button span {
      display: inline;
    }

    .workspace {
      flex: 1;
      min-height: 0;
      gap: 9px;
      padding: 11px 12px 12px;
    }

    .toolbar {
      grid-template-columns: minmax(0, 1fr) auto;
      grid-template-areas:
        'title title'
        'secondary primary';
      gap: 8px;
      align-items: center;
    }

    .toolbar-title {
      grid-area: title;
      display: block;
    }

    .toolbar-primary {
      grid-area: primary;
    }

    .toolbar-secondary {
      grid-area: secondary;
    }

    .toolbar-heading {
      justify-content: space-between;
      flex-wrap: nowrap;
      gap: 10px;
    }

    .toolbar-heading h2 {
      white-space: nowrap;
    }

    .toolbar-context {
      flex: 0 0 auto;
      margin: 0;
    }

    .quick-actions {
      justify-content: flex-end;
      gap: 5px;
    }

    .tool-button {
      width: 34px;
      padding: 0;
    }

    .tool-button span {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }

    .toolbar-secondary {
      flex-wrap: nowrap;
      overflow: hidden;
    }

    .date-shortcuts {
      flex: 0 1 auto;
    }

    .search-field {
      min-width: 150px;
    }
  }

  @media (max-width: 600px) {
    .app-shell {
      background: linear-gradient(180deg, #101819 0 118px, #f7fafa 118px, #eef2f3 100%);
    }

    .sidebar {
      grid-template-columns: 1fr;
      grid-template-areas:
        'brand'
        'filters';
    }

    .toolbar {
      grid-template-columns: 1fr;
      grid-template-areas:
        'title'
        'secondary'
        'primary';
      align-items: stretch;
    }

    .toolbar-heading {
      justify-content: space-between;
      flex-wrap: wrap;
    }

    .toolbar-secondary {
      flex-wrap: wrap;
      overflow: visible;
    }

    .toolbar-primary,
    .quick-actions {
      justify-content: stretch;
    }

    .tool-button {
      width: auto;
      flex: 1 1 0;
      padding: 0 10px;
    }

    .tool-button span {
      position: static;
      width: auto;
      height: auto;
      padding: 0;
      overflow: visible;
      clip: auto;
      white-space: nowrap;
    }
  }

  @media (max-width: 720px) {
    h2 {
      font-size: 1.2rem;
    }

    .tool-button,
    .icon-tool {
      min-height: 32px;
    }

    .day-field {
      width: 130px;
      flex: 0 0 130px;
      min-height: 33px;
    }

    .search-field {
      width: auto;
      flex: 1 1 190px;
      min-height: 34px;
    }

    .history-panel {
      border-radius: 11px;
    }

    .loading-stack {
      padding: 12px;
    }

    .empty-state {
      min-height: 320px;
      padding: 28px;
    }

    .empty-mark {
      width: 62px;
      height: 62px;
      border-radius: 16px;
    }

    .item {
      padding: 11px 12px;
    }

    .item-row {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .item-actions button {
      width: 28px;
      height: 28px;
    }
  }

  /* Compact utility pass: keep the main surface light, dense, and predictable. */
  .app-shell {
    grid-template-columns: 164px minmax(0, 1fr);
  }

  .sidebar {
    gap: 8px;
    padding: 10px;
  }

  .brand {
    gap: 8px;
    padding: 0 0 8px;
  }

  .brand-mark {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    box-shadow: none;
  }

  h1 {
    font-size: 0.95rem;
  }

  .brand p {
    display: none;
  }

  .filter-button {
    min-height: 30px;
    padding: 6px 8px;
    border-radius: 7px;
    font-size: 0.82rem;
  }

  .workspace {
    gap: 8px;
    padding: 10px 12px;
  }

  .workspace::before {
    display: none;
  }

  .toolbar {
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      'title primary'
      'secondary secondary';
    gap: 6px 8px;
    align-items: center;
  }

  .toolbar-tools {
    display: contents;
  }

  .toolbar-title {
    grid-area: title;
  }

  .toolbar-primary {
    grid-area: primary;
  }

  .toolbar-secondary {
    grid-area: secondary;
    display: grid;
    grid-template-columns: 120px minmax(0, auto) minmax(160px, 1fr);
    gap: 6px;
    align-items: center;
    justify-content: stretch;
  }

  .eyebrow {
    display: none;
  }

  .toolbar-heading {
    gap: 8px;
    margin: 0;
    flex-wrap: nowrap;
  }

  .toolbar-heading h2 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.05;
    white-space: nowrap;
    text-wrap: normal;
  }

  .toolbar-context {
    min-height: 21px;
    padding: 0 7px;
    border-radius: 7px;
    font-size: 0.7rem;
    box-shadow: none;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    box-shadow: none;
  }

  .quick-actions {
    gap: 4px;
  }

  .tool-button,
  .icon-tool,
  .day-field,
  .search-field,
  .date-shortcuts button {
    box-shadow: none;
  }

  .tool-button,
  .icon-tool {
    min-height: 28px;
    border-radius: 7px;
  }

  .tool-button {
    padding: 0 8px;
    font-size: 0.78rem;
  }

  .icon-tool {
    width: 30px;
  }

  .day-field {
    width: auto;
    min-height: 30px;
    flex: 0 0 auto;
    padding: 0 8px;
    border-radius: 7px;
    font-size: 0.8rem;
  }

  .date-shortcuts {
    gap: 4px;
    overflow: hidden;
  }

  .date-shortcuts button {
    min-height: 25px;
    padding: 0 7px;
    border-radius: 7px;
    font-size: 0.72rem;
  }

  .search-field {
    width: auto;
    min-width: 0;
    min-height: 30px;
    flex: 0 1 auto;
    padding: 0 8px;
    border-radius: 7px;
    font-size: 0.82rem;
  }

  .history-panel {
    border-radius: 8px;
    box-shadow: none;
  }

  .item {
    padding: 7px 10px;
  }

  .item::before {
    inset: 8px auto 8px 0;
    width: 2px;
  }

  .item-row {
    gap: 6px;
  }

  .item-meta {
    gap: 5px;
    font-size: 0.72rem;
  }

  .type-pill,
  .badge {
    min-height: 18px;
    padding: 0 5px;
    border-radius: 6px;
    font-size: 0.72rem;
  }

  .item-actions {
    gap: 2px;
  }

  .item-actions button {
    width: 26px;
    height: 26px;
    border-radius: 6px;
  }

  .text-content {
    margin-top: 5px;
    max-height: 3.4em;
    padding: 5px 7px;
    border-radius: 7px;
    font-size: 0.88rem;
    line-height: 1.35;
  }

  .image-summary {
    gap: 6px;
    margin-top: 5px;
    font-size: 0.78rem;
  }

  .image-summary strong {
    font-size: 0.84rem;
  }

  .image-preview {
    max-width: 180px;
    max-height: 96px;
    margin-top: 5px;
    border-radius: 7px;
  }

  .image-preview img {
    max-width: 180px;
    max-height: 96px;
  }

  .annotation-note {
    gap: 3px;
    margin-top: 6px;
    padding: 6px 8px;
    border-radius: 7px;
  }

  @media (max-width: 720px) {
    .app-shell {
      background: linear-gradient(180deg, #101819 0 52px, #f7fafa 52px, #eef2f3 100%);
    }

    .sidebar {
      min-height: 52px;
      grid-template-columns: auto minmax(0, 1fr);
      grid-template-areas: 'brand filters';
      gap: 8px;
      padding: 7px 10px;
      box-shadow: none;
    }

    .brand {
      gap: 7px;
    }

    .brand-mark {
      width: 28px;
      height: 28px;
    }

    h1 {
      font-size: 0.9rem;
    }

    .filter-nav {
      gap: 4px;
    }

    .filter-button {
      min-height: 28px;
      justify-content: center;
      gap: 5px;
      padding: 4px 6px;
      font-size: 0.8rem;
    }

    .filter-button :global(svg) {
      width: 14px;
      height: 14px;
    }

    .workspace {
      gap: 7px;
      padding: 8px 10px 10px;
    }

    .toolbar {
      grid-template-columns: minmax(0, 1fr) auto;
      grid-template-areas:
        'title primary'
        'secondary secondary';
      gap: 6px;
      align-items: center;
    }

    .toolbar-heading {
      justify-content: flex-start;
      gap: 7px;
      flex-wrap: nowrap;
    }

    .toolbar-heading h2 {
      font-size: 0.98rem;
    }

    .toolbar-context {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .quick-actions {
      justify-content: flex-end;
    }

    .tool-button {
      width: 28px;
      padding: 0;
    }

    .tool-button span {
      display: none;
    }

    .icon-tool {
      width: 28px;
    }

    .toolbar-secondary {
      grid-template-columns: 112px minmax(0, auto) minmax(130px, 1fr);
      gap: 5px;
      overflow: visible;
    }

    .day-field {
      min-height: 29px;
      width: auto;
      flex: 0 0 auto;
      padding: 0 7px;
    }

    .date-shortcuts {
      min-width: 0;
    }

    .date-shortcuts button:nth-child(n + 3) {
      display: none;
    }

    .search-field {
      min-height: 29px;
      width: auto;
      min-width: 0;
    }

    .history-panel {
      border-radius: 8px;
    }

    .item {
      padding: 7px 8px;
    }

    .item-actions button {
      width: 25px;
      height: 25px;
    }

    .image-preview,
    .image-preview img {
      max-width: 160px;
      max-height: 82px;
    }
  }

  @media (max-width: 460px) {
    .toolbar {
      grid-template-columns: 1fr;
      grid-template-areas:
        'title'
        'secondary'
        'primary';
    }

    .toolbar-secondary {
      grid-template-columns: minmax(0, 1fr) auto;
    }

    .search-field {
      grid-column: 1 / -1;
    }

    .toolbar-primary,
    .quick-actions {
      justify-content: stretch;
    }

    .tool-button {
      width: auto;
      flex: 1 1 0;
    }

    .tool-button span {
      display: inline;
    }
  }
</style>
