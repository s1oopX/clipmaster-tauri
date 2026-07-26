<script>
  import { onDestroy, onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import {
    clipboardApi,
    sessionApi,
    searchApi,
    toolApi,
    settingsApi,
    convertImagePath,
  } from './lib/api.js';
  import AppShell from './components/AppShell.svelte';
  import PinShell from './components/PinShell.svelte';
  import {
    defaultSettings,
    filters,
  } from './lib/app-config.js';
  import {
    deleteReasonLabel,
    limitItems as limitRecordItems,
    pageSize as configuredPageSize,
  } from './lib/app-helpers.js';
  import {
    isActivationKey,
    itemMatchesSearchQuery,
    itemLabel,
  } from './lib/clipboard-ui.js';
  import { createContextMenuController } from './lib/context-menu-controller.js';
  import { createDatePickerAction } from './lib/date-picker-action.js';
  import { createImagePreviewController } from './lib/image-preview-controller.js';
  import { createItemEditController } from './lib/item-edit-controller.js';
  import { createLinkActionsController } from './lib/link-actions-controller.js';
  import { createRecordActionsController } from './lib/record-actions-controller.js';
  import { createRecordsController } from './lib/records-controller.js';

  let items = [];
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
  let settingsSaving = false;
  let monitorToggleSaving = false;
  let appSettings = { ...defaultSettings };
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
  let hasMoreRecords = false;
  let loadingMore = false;
  const contextMenuActions = createContextMenuController({
    getContextMenu: () => contextMenu,
    setContextMenu: (nextContextMenu) => {
      contextMenu = nextContextMenu;
    },
    getWindow: () => window,
  });
  const imagePreviewController = createImagePreviewController({
    convertImagePath,
    getItems: () => items,
    getImageUrls: () => imageUrls,
    setImageUrls: (nextImageUrls) => {
      imageUrls = nextImageUrls;
    },
    getThumbnailUrls: () => thumbnailUrls,
    setThumbnailUrls: (nextThumbnailUrls) => {
      thumbnailUrls = nextThumbnailUrls;
    },
    getImagePreviewErrors: () => imagePreviewErrors,
    setImagePreviewErrors: (nextImagePreviewErrors) => {
      imagePreviewErrors = nextImagePreviewErrors;
    },
    setViewingImageId: (itemId) => {
      viewingImageId = itemId;
    },
    showActionError,
  });
  const recordsController = createRecordsController({
    clipboardApi,
    searchApi,
    defaultSettings,
    getActiveFilter: () => activeFilter,
    getAppSettings: () => appSettings,
    getItems: () => items,
    setItems: (nextItems) => {
      items = nextItems;
    },
    getSearchQuery: () => searchQuery,
    setSearchQuery: (query) => {
      searchQuery = query;
    },
    getSelectedDay: () => selectedDay,
    setSelectedDay: (day) => {
      selectedDay = day;
    },
    getLoading: () => loading,
    setLoading: (loadingState) => {
      loading = loadingState;
    },
    getLoadingMore: () => loadingMore,
    setLoadingMore: (loadingState) => {
      loadingMore = loadingState;
    },
    getIsSearching: () => isSearching,
    setIsSearching: (searching) => {
      isSearching = searching;
    },
    getHasMoreRecords: () => hasMoreRecords,
    setHasMoreRecords: (hasMore) => {
      hasMoreRecords = hasMore;
    },
    setError: (nextError) => {
      error = nextError;
    },
    clearActionError: () => {
      actionError = '';
    },
    showActionError,
    pruneImageUrls: imagePreviewController.pruneImageUrls,
    reconcileTransientItemState,
    loadImageUrls: imagePreviewController.loadImageUrls,
  });
  const datePicker = createDatePickerAction(recordsController.selectDay);
  const linkActions = createLinkActionsController({
    toolApi,
    showActionError,
  });
  const recordActions = createRecordActionsController({
    clipboardApi,
    toolApi,
    getItems: () => items,
    setItems: (nextItems) => {
      items = nextItems;
    },
    getVisibleItems: () => filteredItems,
    getDeleteCandidate: () => deleteCandidate,
    getDeleteConfirmLoading: () => deleteConfirmLoading,
    setDeleteCandidate: (candidate) => {
      deleteCandidate = candidate;
    },
    setDeleteConfirmLoading: (loadingState) => {
      deleteConfirmLoading = loadingState;
    },
    setToolLoading: (loadingTool) => {
      toolLoading = loadingTool;
    },
    setError: (nextError) => {
      error = nextError;
    },
    clearActionError: () => {
      actionError = '';
    },
    showActionError,
    showActionNotice,
    showCopyToast,
    loadAvailableDays,
    pruneImageUrls: imagePreviewController.pruneImageUrls,
    reconcileTransientItemState,
    updateVisibleItem,
  });
  const itemEditActions = createItemEditController({
    clipboardApi,
    getEditContent: () => editContent,
    getAnnotationDraft: () => annotationDraft,
    setEditingId: (itemId) => {
      editingId = itemId;
    },
    setEditContent: (content) => {
      editContent = content;
    },
    setAnnotationEditingId: (itemId) => {
      annotationEditingId = itemId;
    },
    setAnnotationDraft: (annotation) => {
      annotationDraft = annotation;
    },
    showActionError,
    showActionNotice,
    updateVisibleItem,
  });

  $: activeContextItem = contextMenu.open
    ? items.find((item) => item.id === contextMenu.itemId) || null
    : null;

  $: filteredItems = items;

  $: recordsScope = selectedDay || '全部日期';

  function itemMatchesLiveScope(item) {
    const query = searchQuery.trim();

    if (selectedDay && item.date_key !== selectedDay) {
      return false;
    }

    return itemMatchesSearchQuery(item, query);
  }

  function closeImageViewerFromKeyboard(event) {
    if (event.key !== 'Escape' && !isActivationKey(event)) return;
    event.preventDefault();
    viewingImageId = null;
  }

  function focusSearchFromHotkey() {
    settingsOpen = false;
    contextMenuActions.closeContextMenu();

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

      await sessionApi.getCurrentSession();
      await loadAvailableDays();
      await recordsController.refreshVisibleRecords();
      document.addEventListener('click', contextMenuActions.handleDocumentClick);
      document.addEventListener('keydown', contextMenuActions.handleDocumentKeyDown);

      // 监听快捷键事件
      unlistenHotkeys = [
        await listen('hotkey:screenshot', async () => {
          await recordActions.startScreenshot();
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
          recordActions.sortItems();
          reconcileTransientItemState(items);

          await imagePreviewController.ensureImagePreviewUrl(item);

          imagePreviewController.pruneImageUrls(items);
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

    unlistenHotkeys.forEach((unlisten) => {
      if (typeof unlisten === 'function') unlisten();
    });

    if (copyTimer) clearTimeout(copyTimer);
    if (noticeTimer) clearTimeout(noticeTimer);
    if (errorNoticeTimer) clearTimeout(errorNoticeTimer);
    recordsController.dispose();
    document.removeEventListener('click', contextMenuActions.handleDocumentClick);
    document.removeEventListener('keydown', contextMenuActions.handleDocumentKeyDown);
  });

  async function loadAvailableDays() {
    try {
      availableDays = await clipboardApi.getAvailableDays(365);
    } catch (e) {
      console.error('加载日期列表失败:', e);
      availableDays = [];
    }
  }

  function openSettings() {
    settingsOpen = true;
  }

  async function selectFilter(filterId) {
    if (activeFilter === filterId) return;
    activeFilter = filterId;
    await recordsController.refreshVisibleRecords();
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
      showActionNotice(enabled ? '已恢复剪贴板记录' : '已暂停剪贴板记录');
    } catch (e) {
      console.error('切换剪贴板监听失败:', e);
      showActionError('切换剪贴板监听失败: ' + e);
    } finally {
      monitorToggleSaving = false;
    }
  }

  async function refreshSettingsRecords() {
    await loadAvailableDays();
    await recordsController.refreshVisibleRecords();
  }

  async function handleHistoryCleared(plan) {
    searchQuery = '';
    items = [];
    imageUrls = {};
    thumbnailUrls = {};
    viewingImageId = null;
    deleteCandidate = null;
    contextMenuActions.closeContextMenu();
    recordsController.invalidateRequests();

    await sessionApi.getCurrentSession();
    await refreshSettingsRecords();
    showActionNotice(
      plan.item_count > 0
        ? `已清空 ${plan.item_count} 条记录`
        : '没有需要清空的记录'
    );
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

  function limitItems(nextItems) {
    return limitRecordItems(nextItems, configuredPageSize(appSettings, defaultSettings));
  }

  function reconcileTransientItemState(nextItems = items) {
    const liveIds = new Set(nextItems.map((item) => item.id));

    if (contextMenu.open && !liveIds.has(contextMenu.itemId)) {
      contextMenuActions.closeContextMenu();
    }

    if (deleteCandidate && !liveIds.has(deleteCandidate.id)) {
      deleteCandidate = null;
      deleteConfirmLoading = false;
    }

    if (editingId && !liveIds.has(editingId)) {
      itemEditActions.cancelContentEdit();
    }

    if (annotationEditingId && !liveIds.has(annotationEditingId)) {
      itemEditActions.cancelAnnotationEdit();
    }

    if (viewingImageId && !liveIds.has(viewingImageId)) {
      viewingImageId = null;
    }
  }

  function updateVisibleItem(itemId, updater) {
    const nextItems = items.map((item) =>
      item.id === itemId ? updater(item) : item
    );

    items = searchQuery.trim()
      ? nextItems.filter(itemMatchesLiveScope)
      : nextItems;
    imagePreviewController.pruneImageUrls(items);
    reconcileTransientItemState(items);
  }

</script>

{#if pinMode}
  <PinShell {pinImagePath} {pinImageUrl} onClose={closePinWindow} />
{:else}
  <AppShell
    bind:annotationDraft
    bind:appSettings
    bind:editContent
    bind:searchInput
    bind:searchQuery
    bind:selectedDay
    bind:settingsOpen
    bind:settingsSaving
    {activeContextItem}
    {activeFilter}
    {actionError}
    {actionNotice}
    {annotationEditingId}
    {availableDays}
    {contextMenu}
    {copySuccess}
    {datePicker}
    {deleteCandidate}
    {deleteConfirmLoading}
    {deleteReasonLabel}
    {editingId}
    {error}
    {filteredItems}
    {filters}
    {hasMoreRecords}
    {imagePreviewErrors}
    {imageUrls}
    {isSearching}
    {itemLabel}
    {loading}
    {loadingMore}
    {monitorToggleSaving}
    {recordsScope}
    {thumbnailUrls}
    {toolLoading}
    {viewingImageId}
    onCancelAnnotationEdit={itemEditActions.cancelAnnotationEdit}
    onCancelContentEdit={itemEditActions.cancelContentEdit}
    onCancelDeleteConfirmation={recordActions.cancelDeleteConfirmation}
    onClearDayFilter={recordsController.clearDayFilter}
    onClearSearch={recordsController.clearSearch}
    onCloseImageViewer={() => (viewingImageId = null)}
    onCloseImageViewerFromKeyboard={closeImageViewerFromKeyboard}
    onConfirmDeleteCandidate={recordActions.confirmDeleteCandidate}
    onCopyItem={recordActions.copyItem}
    onFallbackToOriginalPreview={imagePreviewController.fallbackToOriginalPreview}
    onHistoryCleared={handleHistoryCleared}
    onLoadMoreRecords={recordsController.loadMoreRecords}
    onOpenContextMenu={contextMenuActions.openContextMenu}
    onOpenLink={linkActions.openLinkUrl}
    onOpenRecordLink={linkActions.openRecordLink}
    onOpenSettings={openSettings}
    onPinImageToDesktop={recordActions.pinImageToDesktop}
    onPinNewestImage={recordActions.pinNewestImage}
    onQueueSearch={recordsController.queueSearch}
    onRecordLinkKeyDown={linkActions.handleRecordLinkKeyDown}
    onRefreshSettingsRecords={refreshSettingsRecords}
    onRequestDeleteItem={recordActions.requestDeleteItem}
    onSaveAnnotation={itemEditActions.saveAnnotation}
    onSaveContentEdit={itemEditActions.saveContentEdit}
    onSelectDay={recordsController.selectDay}
    onSelectFilter={selectFilter}
    onStartAnnotationEdit={itemEditActions.startAnnotationEdit}
    onStartContentEdit={itemEditActions.startContentEdit}
    onStartScreenshot={recordActions.startScreenshot}
    onToggleFavorite={recordActions.toggleFavorite}
    onToggleMonitoring={setClipboardMonitoring}
    onTogglePinned={recordActions.togglePinned}
    onViewFullImage={imagePreviewController.viewFullImage}
    runContextAction={contextMenuActions.runContextAction}
    {showActionError}
    {showActionNotice}
  />
{/if}

