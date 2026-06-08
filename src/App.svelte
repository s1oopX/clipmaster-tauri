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
    Image as ImageIcon,
    Inbox,
    LoaderCircle,
    Pause,
    Pin,
    Play,
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
  import ContextMenu from './components/ContextMenu.svelte';
  import DeleteConfirmDialog from './components/DeleteConfirmDialog.svelte';
  import ImageViewer from './components/ImageViewer.svelte';
  import PinShell from './components/PinShell.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import ToastStack from './components/ToastStack.svelte';
  import {
    appVersion,
    defaultSettings,
    filters,
    githubIssuesUrl,
    githubProfileUrl,
    languageOptions,
    settingsViews,
    timeZoneOptions,
  } from './lib/app-config.js';
  import {
    formatDateKey,
    formatTime,
    isActivationKey,
    itemLabel,
    itemMatchesSearchQuery,
    runKeyboardAction,
    todayDateKey,
  } from './lib/clipboard-ui.js';

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
  let monitorToggleSaving = false;
  let cleanupLoading = false;
  let cleanupPlan = null;
  let clearHistoryConfirmOpen = false;
  let clearHistoryLoading = false;
  let portCheckLoading = false;
  let portCheckResult = null;
  let pendingRestartPort = null;
  let restartingApp = false;
  let appSettings = { ...defaultSettings };
  let settingsDraft = { ...defaultSettings };
  let appDataDir = '';
  let appDataDirError = '';
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

  function activeSearchDateKey() {
    return selectedDay || todayDateKey(appSettings.time_zone);
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
      await loadAppDataDir();

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

  async function loadAppDataDir() {
    try {
      appDataDir = await settingsApi.getAppDataDir();
      appDataDirError = '';
    } catch (e) {
      console.error('读取数据目录失败:', e);
      appDataDir = '';
      appDataDirError = e.toString();
    }
  }

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

  async function setClipboardMonitoring(enabled) {
    if (monitorToggleSaving || settingsSaving) return;
    monitorToggleSaving = true;
    error = null;

    try {
      const savedSettings = await settingsApi.saveSettings({
        ...appSettings,
        clipboard_monitor_enabled: enabled,
      });
      appSettings = savedSettings;
      settingsDraft = settingsOpen
        ? {
            ...settingsDraft,
            clipboard_monitor_enabled: savedSettings.clipboard_monitor_enabled,
          }
        : { ...savedSettings };
      showActionNotice(enabled ? '已恢复剪贴板记录' : '已暂停剪贴板记录');
    } catch (e) {
      console.error('切换剪贴板监听失败:', e);
      showActionError('切换剪贴板监听失败: ' + e);
    } finally {
      monitorToggleSaving = false;
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

  function requestClearAllHistory() {
    cleanupPlan = null;
    closeContextMenu();
    clearHistoryConfirmOpen = true;
  }

  function cancelClearAllHistory() {
    if (clearHistoryLoading) return;
    clearHistoryConfirmOpen = false;
  }

  async function confirmClearAllHistory() {
    if (clearHistoryLoading) return;

    clearHistoryLoading = true;
    error = null;

    try {
      const plan = await settingsApi.clearAllHistory();
      clearHistoryConfirmOpen = false;
      selectedDay = '';
      searchQuery = '';
      items = [];
      imageUrls = {};
      thumbnailUrls = {};
      viewingImageId = null;
      deleteCandidate = null;
      closeContextMenu();
      recordsRequestId++;

      currentSession = await sessionApi.getCurrentSession();
      await loadAvailableDays();
      await refreshVisibleRecords();
      showActionNotice(
        plan.item_count > 0
          ? `已清空 ${plan.item_count} 条记录`
          : '没有需要清空的记录'
      );
    } catch (e) {
      console.error('清空历史失败:', e);
      showActionError('清空历史失败: ' + e);
    } finally {
      clearHistoryLoading = false;
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
        is_favorite: savedAnnotation ? true : item.is_favorite,
      }));

      annotationEditingId = null;
      annotationDraft = '';
      showActionNotice(savedAnnotation ? '标注已保存' : '标注已清除');
    } catch (e) {
      console.error('保存标注失败:', e);
      showActionError('保存标注失败: ' + e);
    }
  }

</script>

{#if pinMode}
  <PinShell {pinImagePath} {pinImageUrl} onClose={closePinWindow} />
{:else}
<main class="app-shell" data-testid="app-shell" data-layout="compact-ready" data-density="tool">
  <Sidebar {activeFilter} {filters} onFilterChange={(filterId) => (activeFilter = filterId)} />

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
              class="tool-button monitor-toggle"
              class:paused={!appSettings.clipboard_monitor_enabled}
              on:click={() => setClipboardMonitoring(!appSettings.clipboard_monitor_enabled)}
              aria-pressed={!appSettings.clipboard_monitor_enabled}
              disabled={monitorToggleSaving || settingsSaving}
            >
              {#if monitorToggleSaving}
                <LoaderCircle size={15} aria-hidden="true" />
                <span>保存中</span>
              {:else if appSettings.clipboard_monitor_enabled}
                <Pause size={15} aria-hidden="true" />
                <span>暂停</span>
              {:else}
                <Play size={15} aria-hidden="true" />
                <span>恢复</span>
              {/if}
            </button>

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

    <ToastStack {copySuccess} {actionNotice} {actionError} />
  </section>
  <ContextMenu
    {activeContextItem}
    {contextMenu}
    {runContextAction}
    onAddAnnotation={startAnnotationEdit}
    onCopy={copyItem}
    onEditContent={startContentEdit}
  />
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
              class="settings-section settings-view advanced-settings"
              id="settings-view-advanced"
              role="tabpanel"
              aria-labelledby="settings-tab-advanced"
            >
              <section class="settings-card advanced-card" aria-label="记录清理设置">
                <header class="settings-card-header">
                  <div>
                    <h3 id="cleanup-settings-title">记录清理</h3>
                    <p>只清理普通记录，置顶和收藏会保留。</p>
                  </div>
                  <label class="switch-row compact-switch">
                    <input
                      type="checkbox"
                      aria-label="保存设置后自动清理"
                      checked={settingsDraft.auto_cleanup_enabled}
                      on:change={(event) =>
                        updateSettingsDraft('auto_cleanup_enabled', event.currentTarget.checked)}
                    />
                    <span>自动清理</span>
                  </label>
                </header>

                <div class="settings-field-grid">
                  <label class="field-row compact-field">
                    <span>最多保留</span>
                    <input
                      type="number"
                      aria-label="普通记录最多保留"
                      min="10"
                      max="5000"
                      value={settingsDraft.cleanup_max_items}
                      on:input={(event) =>
                        updateSettingsDraft('cleanup_max_items', Number(event.currentTarget.value))}
                    />
                  </label>

                  <label class="field-row compact-field">
                    <span>保留天数</span>
                    <input
                      type="number"
                      aria-label="普通记录保留天数"
                      min="1"
                      max="3650"
                      value={settingsDraft.cleanup_keep_days}
                      on:input={(event) =>
                        updateSettingsDraft('cleanup_keep_days', Number(event.currentTarget.value))}
                    />
                  </label>
                </div>

                <p class="cleanup-hint">图片文件会随被清理的图片记录同步删除。</p>

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
              </section>

              <section class="settings-card advanced-card danger-card" aria-label="危险操作">
                <header class="settings-card-header">
                  <div>
                    <h3>危险操作</h3>
                    <p>清空全部记录、收藏、置顶、标注和图片文件。</p>
                  </div>
                </header>

                <div class="danger-actions">
                  <p class="danger-copy">这个操作无法撤销，当前窗口会立即刷新为空历史。</p>
                  <button
                    type="button"
                    class="danger-button clear-history-button"
                    on:click={requestClearAllHistory}
                    disabled={clearHistoryLoading}
                  >
                    <Trash2 size={15} aria-hidden="true" />
                    {clearHistoryLoading ? '清空中' : '清空全部历史'}
                  </button>
                </div>
              </section>

              <section class="settings-card advanced-card" aria-label="开发端口设置">
                <header class="settings-card-header">
                  <div>
                    <h3 id="port-settings-title">开发端口</h3>
                    <p>检查占用状态，保存后重启生效。</p>
                  </div>
                </header>

                <div class="field-row port-field compact-field">
                  <label for="dev-server-port">端口</label>
                  <div class="port-input-group">
                    <input
                      id="dev-server-port"
                      type="number"
                      aria-label="开发端口"
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
                    <p>生产版不会占用本地开发端口。</p>
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
              </section>
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
                  <dt>数据目录</dt>
                  <dd class="path-value">
                    {appDataDir || (appDataDirError ? '读取失败，请查看排障文档' : '加载中')}
                  </dd>
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

  {#if clearHistoryConfirmOpen}
    <button
      type="button"
      class="confirm-backdrop"
      aria-label="取消清空历史确认"
      on:click={cancelClearAllHistory}
    ></button>
    <div
      class="confirm-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="clear-history-confirm-title"
      aria-describedby="clear-history-confirm-desc"
    >
      <header>
        <div class="confirm-icon">
          <Trash2 size={18} aria-hidden="true" />
        </div>
        <div>
          <h2 id="clear-history-confirm-title">确认清空历史</h2>
          <p id="clear-history-confirm-desc">
            所有剪贴板记录、收藏、置顶、标注、图片原图和缩略图都会被删除。
          </p>
        </div>
      </header>
      <div class="confirm-preview">
        清空后无法恢复，当前活动会话会保留为空会话。
      </div>
      <footer>
        <button
          type="button"
          class="ghost-button"
          on:click={cancelClearAllHistory}
          disabled={clearHistoryLoading}
        >
          取消
        </button>
        <button
          type="button"
          class="danger-button"
          on:click={confirmClearAllHistory}
          disabled={clearHistoryLoading}
        >
          {clearHistoryLoading ? '清空中' : '确认清空'}
        </button>
      </footer>
    </div>
  {/if}

  <DeleteConfirmDialog
    {deleteCandidate}
    {deleteConfirmLoading}
    {deleteReasonLabel}
    {itemLabel}
    onCancel={cancelDeleteConfirmation}
    onConfirm={confirmDeleteCandidate}
  />

  <ImageViewer
    {imageUrls}
    {viewingImageId}
    onClose={closeImageViewer}
    onKeyboardClose={closeImageViewerFromKeyboard}
  />
</main>
{/if}

