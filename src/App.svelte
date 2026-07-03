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
    LoaderCircle,
    Pause,
    Pin,
    Play,
    Search,
    Settings,
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
  import ClipboardHistoryPanel from './components/ClipboardHistoryPanel.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import DeleteConfirmDialog from './components/DeleteConfirmDialog.svelte';
  import ImageViewer from './components/ImageViewer.svelte';
  import PinShell from './components/PinShell.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
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
    isActivationKey,
    itemMatchesSearchQuery,
    todayDateKey,
    effectiveItemType,
    itemLabel,
  } from './lib/clipboard-ui.js';

  let items = [];
  let currentSession = null;
  let loading = false;
  let error = null;
  let searchQuery = '';
  let searchInput = null;
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
  let recordingHotkeyField = null;
  let hotkeyRecordingMessage = '';
  let recordingHotkeyTimeout = null;
  let pinMode = false;
  let pinImagePath = '';
  let pinImageUrl = '';
  let unlistenNewItem = null;
  let unlistenHotkeys = [];
  let editingId = null;
  let editContent = '';
  let annotationEditingId = null;
  let annotationDraft = '';
  let contextMenu = { open: false, x: 0, y: 0, itemId: null };
  let activeContextItem = null;
  let deleteCandidate = null;
  let deleteConfirmLoading = false;
  let thumbnailUrls = {};
  let imagePreviewErrors = {};
  let viewingImageId = null;
  let availableDays = [];
  let selectedDay = '';
  let recordsScope = '全部日期';
  let recordsRequestId = 0;
  let hasMoreRecords = false;
  let loadingMore = false;
  let searchTimer = null;

  $: activeContextItem = contextMenu.open
    ? items.find((item) => item.id === contextMenu.itemId) || null
    : null;

  $: filteredItems = items;

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

  function focusSearchFromHotkey() {
    settingsOpen = false;
    stopRecordingHotkey();
    closeContextMenu();

    setTimeout(() => {
      searchInput?.focus();
      searchInput?.select?.();
    }, 0);
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
      unlistenHotkeys = [
        await listen('hotkey:screenshot', async () => {
          await startScreenshot();
        }),
        await listen('hotkey:focus-search', () => {
          focusSearchFromHotkey();
        }),
      ];

      unlistenNewItem = await clipboardApi.onNewItem(async (item) => {
        await loadAvailableDays();

        if (itemMatchesLiveScope(item)) {
          items = limitItems([
            item,
            ...items.filter((existing) => existing.id !== item.id),
          ]);
          sortItems();
          reconcileTransientItemState(items);

          await ensureImagePreviewUrl(item);

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

    unlistenHotkeys.forEach((unlisten) => {
      if (typeof unlisten === 'function') unlisten();
    });

    if (copyTimer) clearTimeout(copyTimer);
    if (noticeTimer) clearTimeout(noticeTimer);
    if (errorNoticeTimer) clearTimeout(errorNoticeTimer);
    if (recordingHotkeyTimeout) clearTimeout(recordingHotkeyTimeout);
    if (searchTimer) clearTimeout(searchTimer);
    document.removeEventListener('click', handleDocumentClick);
    document.removeEventListener('keydown', handleDocumentKeyDown);
  });

  async function loadItems(day = selectedDay, { append = false } = {}) {
    const requestId = ++recordsRequestId;
    isSearching = false;
    if (append) {
      loadingMore = true;
    } else {
      loading = true;
      hasMoreRecords = false;
    }

    try {
      const offset = append ? items.length : 0;
      const limit = pageSize();
      const filter = activeFilterQuery();
      const hasFilter = Object.keys(filter).length > 0;
      const nextItems = day
        ? hasFilter
          ? await clipboardApi.getItemsByDay(day, limit, offset, filter)
          : await clipboardApi.getItemsByDay(day, limit, offset)
        : hasFilter
          ? await clipboardApi.getItems(limit, offset, filter)
          : await clipboardApi.getItems(limit, offset);

      if (requestId !== recordsRequestId) return;

      items = append ? mergeItems(items, nextItems) : nextItems;
      hasMoreRecords = nextItems.length === limit;
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
        loadingMore = false;
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
      if (item.type === 'image' && !thumbnailUrls[item.id]) {
        await ensureImagePreviewUrl(item);
      }
    }

    thumbnailUrls = thumbnailUrls;
    imagePreviewErrors = imagePreviewErrors;
  }

  async function ensureImagePreviewUrl(item) {
    if (item.type !== 'image' || (!item.thumbnail_path && !item.image_path)) {
      return;
    }

    const previewUrl = await resolveFirstImageUrl([item.thumbnail_path, item.image_path]);
    if (previewUrl) {
      thumbnailUrls[item.id] = previewUrl;
      delete imagePreviewErrors[item.id];
    } else {
      delete thumbnailUrls[item.id];
      imagePreviewErrors[item.id] = true;
    }

    thumbnailUrls = thumbnailUrls;
    imagePreviewErrors = imagePreviewErrors;
  }

  async function resolveFirstImageUrl(paths) {
    const seen = new Set();

    for (const path of paths) {
      if (!path || seen.has(path)) continue;
      seen.add(path);

      try {
        const url = await convertImagePath(path);
        if (url) return url;
      } catch (e) {
        console.error('加载图片预览 URL 失败:', e);
      }
    }

    return null;
  }

  async function fallbackToOriginalPreview(item) {
    if (!item?.image_path || thumbnailUrls[item.id] === imageUrls[item.id]) {
      delete thumbnailUrls[item.id];
      imagePreviewErrors[item.id] = true;
      thumbnailUrls = thumbnailUrls;
      imagePreviewErrors = imagePreviewErrors;
      return;
    }

    try {
      const originalUrl = imageUrls[item.id] || await resolveFirstImageUrl([item.image_path]);
      if (!originalUrl) throw new Error('原图 URL 不可用');

      imageUrls[item.id] = originalUrl;
      thumbnailUrls[item.id] = originalUrl;
      delete imagePreviewErrors[item.id];
      imageUrls = imageUrls;
      thumbnailUrls = thumbnailUrls;
      imagePreviewErrors = imagePreviewErrors;
    } catch (e) {
      console.error('原图预览加载失败:', e);
      delete thumbnailUrls[item.id];
      imagePreviewErrors[item.id] = true;
      thumbnailUrls = thumbnailUrls;
      imagePreviewErrors = imagePreviewErrors;
      showActionError('图片预览不可用');
    }
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

  async function handleSearch({ append = false } = {}) {
    const query = searchQuery.trim();

    if (!query) {
      await loadItems(selectedDay, { append });
      return;
    }

    const requestId = ++recordsRequestId;
    if (append) {
      loadingMore = true;
    } else {
      loading = false;
      isSearching = true;
      hasMoreRecords = false;
    }

    try {
      const dateKey = activeSearchDateKey();
      const limit = pageSize();
      const offset = append ? items.length : 0;
      const filter = activeFilterQuery();
      const nextItems = Object.keys(filter).length > 0
        ? await searchApi.searchItems(query, null, limit, dateKey, offset, filter)
        : await searchApi.searchItems(query, null, limit, dateKey, offset);

      if (requestId !== recordsRequestId) return;

      items = append ? mergeItems(items, nextItems) : nextItems;
      hasMoreRecords = nextItems.length === limit;
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
        loadingMore = false;
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
    if (searchTimer) clearTimeout(searchTimer);
    searchQuery = '';
    loadItems();
  }

  async function copyItem(item) {
    try {
      if ((item.type === 'text' || effectiveItemType(item) === 'link') && item.content) {
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
    await openLinkUrl(url);
  }

  async function openRecordLink(event, item) {
    if (!event.ctrlKey && !event.metaKey) {
      return;
    }

    event.preventDefault();
    await openLinkUrl(item.content);
  }

  function handleRecordLinkKeyDown(event, item) {
    if (!isActivationKey(event)) return;
    event.preventDefault();
    void openLinkUrl(item.content);
  }

  async function openLinkUrl(url) {
    if (!url) return;

    try {
      await toolApi.openExternalUrl(url);
    } catch (e) {
      console.error('打开链接失败:', e);
      showActionError('打开链接失败: ' + e);
    }
  }

  function queueSearch() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void handleSearch();
    }, 200);
  }

  function activeFilterQuery() {
    if (activeFilter === 'favorite') {
      return { favoriteOnly: true };
    }

    if (activeFilter === 'image') {
      return { itemType: 'image' };
    }

    if (activeFilter === 'link') {
      return { itemType: 'link' };
    }

    return {};
  }

  async function selectFilter(filterId) {
    if (activeFilter === filterId) return;
    activeFilter = filterId;
    await refreshVisibleRecords();
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
      auto_start_enabled: settingsDraft.auto_start_enabled,
      max_items: numberSettingValue(settingsDraft.max_items, defaultSettings.max_items),
      capture_delay_ms: numberSettingValue(
        settingsDraft.capture_delay_ms,
        defaultSettings.capture_delay_ms
      ),
      screenshot_hotkey: settingsDraft.screenshot_hotkey || defaultSettings.screenshot_hotkey,
      main_window_hotkey:
        settingsDraft.main_window_hotkey || defaultSettings.main_window_hotkey,
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
  function startRecordingHotkey(field) {
    isRecordingHotkey = true;
    recordingHotkeyField = field;
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
    recordingHotkeyField = null;
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
    updateSettingsDraft(recordingHotkeyField || 'screenshot_hotkey', hotkey);
    hotkeyRecordingMessage = '';

    // 停止录制
    stopRecordingHotkey();
  }

  function visibleItems() {
    return filteredItems;
  }

  async function loadMoreRecords() {
    if (loading || loadingMore || isSearching || !hasMoreRecords) return;

    if (searchQuery.trim()) {
      await handleSearch({ append: true });
    } else {
      await loadItems(selectedDay, { append: true });
    }
  }

  function pageSize() {
    return appSettings.max_items || defaultSettings.max_items;
  }

  function limitItems(nextItems) {
    return nextItems.slice(0, pageSize());
  }

  function mergeItems(existingItems, nextItems) {
    const seen = new Set(existingItems.map((item) => item.id));
    return [
      ...existingItems,
      ...nextItems.filter((item) => {
        if (seen.has(item.id)) return false;
        seen.add(item.id);
        return true;
      }),
    ];
  }

  function pruneImageUrls(nextItems = items) {
    const liveIds = new Set(nextItems.map((item) => item.id));

    imageUrls = Object.fromEntries(
      Object.entries(imageUrls).filter(([itemId]) => liveIds.has(itemId))
    );

    thumbnailUrls = Object.fromEntries(
      Object.entries(thumbnailUrls).filter(([itemId]) => liveIds.has(itemId))
    );

    imagePreviewErrors = Object.fromEntries(
      Object.entries(imagePreviewErrors).filter(([itemId]) => liveIds.has(itemId))
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

  async function viewFullImage(itemId) {
    const item = items.find((entry) => entry.id === itemId);
    if (item?.image_path && !imageUrls[itemId]) {
      try {
        imageUrls[itemId] = await convertImagePath(item.image_path);
        imageUrls = imageUrls;
      } catch (e) {
        console.error('加载原图 URL 失败:', e);
        showActionError('加载原图失败: ' + e);
        return;
      }
    }

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
      const updatedItem = await clipboardApi.updateItemContent(itemId, editContent);
      updateVisibleItem(itemId, () => updatedItem);

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
<main
  class="app-shell"
  data-testid="app-shell"
  data-layout="compact-ready"
  data-density="tool"
  data-reference="figma-utility-grid"
>
  <Sidebar {activeFilter} {filters} onFilterChange={selectFilter} />

  <section class="workspace" aria-label="剪贴板历史">
    <header class="toolbar">
      <div class="toolbar-title">
        <div class="toolbar-heading">
          <h2>剪贴板历史</h2>
          <p class="toolbar-context" aria-label="当前范围">
            <span class="status-dot"></span>
            {recordsScope} · 已加载 {filteredItems.length} 条
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
              bind:this={searchInput}
              bind:value={searchQuery}
              on:input={queueSearch}
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

    <ClipboardHistoryPanel
      bind:annotationDraft
      bind:editContent
      {annotationEditingId}
      {editingId}
      {filteredItems}
      {hasMoreRecords}
      {imagePreviewErrors}
      {isSearching}
      {loading}
      {loadingMore}
      {searchQuery}
      {thumbnailUrls}
      onCancelAnnotationEdit={cancelAnnotationEdit}
      onCancelContentEdit={cancelContentEdit}
      onCopyItem={copyItem}
      onFallbackToOriginalPreview={fallbackToOriginalPreview}
      onLoadMoreRecords={loadMoreRecords}
      onOpenContextMenu={openContextMenu}
      onOpenLink={openLinkUrl}
      onOpenRecordLink={openRecordLink}
      onPinImageToDesktop={pinImageToDesktop}
      onRecordLinkKeyDown={handleRecordLinkKeyDown}
      onRequestDeleteItem={requestDeleteItem}
      onSaveAnnotation={saveAnnotation}
      onSaveContentEdit={saveContentEdit}
      onStartAnnotationEdit={startAnnotationEdit}
      onToggleFavorite={toggleFavorite}
      onTogglePinned={togglePinned}
      onViewFullImage={viewFullImage}
    />

    <ToastStack {copySuccess} {actionNotice} {actionError} />
  </section>
  <ContextMenu
    {activeContextItem}
    {contextMenu}
    {runContextAction}
    onAddAnnotation={startAnnotationEdit}
    onCopy={copyItem}
    onEditContent={startContentEdit}
    onOpenLink={(item) => openLinkUrl(item.content)}
  />
  {#if settingsOpen}
    <SettingsPanel
      bind:activeSettingsView
      {appDataDir}
      {appDataDirError}
      {appVersion}
      {cleanupLoading}
      {cleanupPlan}
      {clearHistoryLoading}
      {githubIssuesUrl}
      {githubProfileUrl}
      {hotkeyRecordingMessage}
      {isRecordingHotkey}
      {languageOptions}
      {pendingRestartPort}
      {portCheckLoading}
      {portCheckResult}
      {recordingHotkeyField}
      {restartingApp}
      {settingsDraft}
      {settingsPortChanged}
      {settingsSaving}
      {settingsViews}
      {timeZoneOptions}
      onApplySuggestedPort={applySuggestedPort}
      onCheckDevServerPort={checkDevServerPort}
      onClose={() => (settingsOpen = false)}
      onHotkeyBlur={stopRecordingHotkey}
      onHotkeyFocus={startRecordingHotkey}
      onHotkeyKeyDown={handleHotkeyKeyDown}
      onOpenExternalLink={openExternalLink}
      onPreviewCleanup={previewCleanup}
      onRequestClearAllHistory={requestClearAllHistory}
      onRestartApplication={restartApplication}
      onRunCleanupNow={runCleanupNow}
      onSaveSettings={saveSettings}
      onUpdateSettingsDraft={updateSettingsDraft}
    />
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

