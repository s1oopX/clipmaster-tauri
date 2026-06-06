<script>
  import { onDestroy, onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import {
    Camera,
    Check,
    Clipboard,
    Copy,
    FileText,
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
    auto_cleanup_enabled: false,
    cleanup_max_items: 200,
    cleanup_keep_days: 30,
  };

  let items = [];
  let currentSession = null;
  let loading = false;
  let error = null;
  let searchQuery = '';
  let isSearching = false;
  let activeFilter = 'all';

  let imageUrls = {};
  let copySuccess = false;
  let copyTimer = null;
  let actionNotice = '';
  let noticeTimer = null;
  let toolLoading = null;
  let settingsOpen = false;
  let settingsSaving = false;
  let cleanupLoading = false;
  let cleanupPlan = null;
  let appSettings = { ...defaultSettings };
  let settingsDraft = { ...defaultSettings };
  let isRecordingHotkey = false;
  let recordingHotkeyTimeout = null;
  let pinMode = false;
  let pinImagePath = '';
  let pinImageUrl = '';
  let unlistenNewItem = null;
  let editingId = null;
  let editContent = '';
  let thumbnailUrls = {};
  let viewingImageId = null;
  let availableDays = [];
  let selectedDay = '';

  const filters = [
    { id: 'all', label: '全部记录' },
    { id: 'favorite', label: '收藏' },
    { id: 'image', label: '图片' },
  ];

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
      await loadItems();

      // 监听快捷键事件
      await listen('hotkey:screenshot', async () => {
        await startScreenshot();
      });

      unlistenNewItem = await clipboardApi.onNewItem(async (item) => {
        await loadAvailableDays();

        if (!selectedDay || item.date_key === selectedDay) {
          const nextItems = limitItems([item, ...items]);
          items = nextItems;

          if (item.type === 'image' && item.image_path) {
            imageUrls[item.id] = await convertImagePath(item.image_path);
          }

          if (item.type === 'image' && item.thumbnail_path) {
            thumbnailUrls[item.id] = await convertImagePath(item.thumbnail_path);
          }

          pruneImageUrls(nextItems);
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

    if (copyTimer) clearTimeout(copyTimer);
    if (noticeTimer) clearTimeout(noticeTimer);
    if (recordingHotkeyTimeout) clearTimeout(recordingHotkeyTimeout);
  });

  async function loadItems() {
    loading = true;
    try {
      items = selectedDay
        ? await clipboardApi.getItemsByDay(selectedDay, itemLimit(), 0)
        : await clipboardApi.getItems(itemLimit(), 0);
      pruneImageUrls(items);
      await loadImageUrls();
    } catch (e) {
      console.error('加载记录失败:', e);
      error = e.toString();
    } finally {
      loading = false;
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

  async function handleDayChange(event) {
    selectedDay = event.currentTarget.value;
    searchQuery = '';
    await loadItems();
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

  async function deleteItem(itemId) {
    try {
      await clipboardApi.deleteItem(itemId);
      items = items.filter((item) => item.id !== itemId);
      pruneImageUrls(items);
    } catch (e) {
      console.error('删除失败:', e);
      error = '删除失败: ' + e;
    }
  }

  async function toggleFavorite(itemId) {
    try {
      const isFavorite = await clipboardApi.toggleFavorite(itemId);
      items = items.map((item) =>
        item.id === itemId ? { ...item, is_favorite: isFavorite } : item
      );
    } catch (e) {
      console.error('切换收藏失败:', e);
      error = '切换收藏失败: ' + e;
    }
  }

  async function togglePinned(itemId) {
    try {
      const isPinned = await clipboardApi.togglePinned(itemId);
      items = items.map((item) =>
        item.id === itemId ? { ...item, is_pinned: isPinned } : item
      );
      sortItems();
    } catch (e) {
      console.error('切换置顶失败:', e);
      error = '切换置顶失败: ' + e;
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
    if (!searchQuery.trim()) {
      await loadItems();
      return;
    }

    isSearching = true;
    try {
      const sessionId = currentSession?.id || null;
      items = await searchApi.searchItems(
        searchQuery,
        sessionId,
        itemLimit()
      );
      pruneImageUrls(items);
      await loadImageUrls();
    } catch (e) {
      console.error('搜索失败:', e);
      error = e.toString();
    } finally {
      isSearching = false;
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
        showCopyToast();
      } else if (item.type === 'image' && item.image_path) {
        await clipboardApi.copyImageToClipboard(item.image_path);
        showCopyToast();
      } else if (item.type === 'image') {
        error = '图片路径不可用';
      }
    } catch (e) {
      console.error('复制失败:', e);
      error = '复制失败: ' + e;
    }
  }

  async function startScreenshot() {
    toolLoading = 'screenshot';
    error = null;

    try {
      // 直接进入全屏选取模式
      await toolApi.startRegionScreenshot();
      toolLoading = null;
    } catch (e) {
      console.error('截图失败:', e);
      error = '截图失败: ' + e;
      toolLoading = null;
    }
  }

  async function pinNewestImage() {
    const image = visibleItems().find((item) => item.type === 'image' && item.image_path)
      || items.find((item) => item.type === 'image' && item.image_path);

    if (!image) {
      error = '当前没有可钉住的图片记录';
      return;
    }

    await pinImageToDesktop(image);
  }

  async function pinImageToDesktop(item) {
    if (!item.image_path) {
      error = '图片路径不可用';
      return;
    }

    toolLoading = 'pin';
    error = null;

    try {
      await toolApi.pinImage(item.image_path);
      showActionNotice('已钉到桌面');
    } catch (e) {
      console.error('贴图失败:', e);
      error = '贴图失败: ' + e;
    } finally {
      toolLoading = null;
    }
  }

  function openSettings() {
    settingsDraft = { ...appSettings };
    cleanupPlan = null;
    settingsOpen = true;
  }

  function updateSettingsDraft(key, value) {
    settingsDraft = {
      ...settingsDraft,
      [key]: value,
    };
  }

  async function saveSettings() {
    settingsSaving = true;
    error = null;

    const normalized = {
      clipboard_monitor_enabled: settingsDraft.clipboard_monitor_enabled,
      show_main_window_on_start: settingsDraft.show_main_window_on_start,
      max_items: Number(settingsDraft.max_items) || defaultSettings.max_items,
      capture_delay_ms: Number(settingsDraft.capture_delay_ms) || defaultSettings.capture_delay_ms,
      screenshot_hotkey: settingsDraft.screenshot_hotkey || defaultSettings.screenshot_hotkey,
      auto_cleanup_enabled: settingsDraft.auto_cleanup_enabled,
      cleanup_max_items: Number(settingsDraft.cleanup_max_items) || defaultSettings.cleanup_max_items,
      cleanup_keep_days: Number(settingsDraft.cleanup_keep_days) || defaultSettings.cleanup_keep_days,
    };

    try {
      appSettings = await settingsApi.saveSettings(normalized);
      settingsDraft = { ...appSettings };
      cleanupPlan = null;
      settingsOpen = false;
      await loadAvailableDays();
      await loadItems();
      showActionNotice('设置已保存');
    } catch (e) {
      console.error('保存设置失败:', e);
      error = '保存设置失败: ' + e;
    } finally {
      settingsSaving = false;
    }
  }

  async function previewCleanup() {
    cleanupLoading = true;
    error = null;

    try {
      cleanupPlan = await settingsApi.previewCustomCleanup(
        Number(settingsDraft.cleanup_max_items) || defaultSettings.cleanup_max_items,
        Number(settingsDraft.cleanup_keep_days) || defaultSettings.cleanup_keep_days
      );
    } catch (e) {
      console.error('预览清理失败:', e);
      error = '预览清理失败: ' + e;
    } finally {
      cleanupLoading = false;
    }
  }

  async function runCleanupNow() {
    cleanupLoading = true;
    error = null;

    try {
      cleanupPlan = await settingsApi.runCustomCleanup(
        Number(settingsDraft.cleanup_max_items) || defaultSettings.cleanup_max_items,
        Number(settingsDraft.cleanup_keep_days) || defaultSettings.cleanup_keep_days
      );
      await loadAvailableDays();
      await loadItems();
      showActionNotice(`已清理 ${cleanupPlan.item_count} 条记录`);
    } catch (e) {
      console.error('执行清理失败:', e);
      error = '执行清理失败: ' + e;
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
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      copySuccess = false;
    }, 1800);
  }

  function showActionNotice(message) {
    actionNotice = message;
    if (noticeTimer) clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      actionNotice = '';
    }, 2200);
  }

  // 快捷键录制相关
  function startRecordingHotkey(event) {
    isRecordingHotkey = true;
    event.target.value = '按下组合键...';

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
      event.target.value = '请使用修饰键组合（如 Ctrl+Shift+A）';
      return;
    }

    parts.push(key);

    const hotkey = parts.join('+');
    updateSettingsDraft('screenshot_hotkey', hotkey);

    // 停止录制
    stopRecordingHotkey();
  }

  function visibleItems() {
    if (activeFilter === 'favorite') {
      return items.filter((item) => item.is_favorite);
    }

    if (activeFilter === 'image') {
      return items.filter((item) => item.type === 'image');
    }

    return items;
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

  function startEdit(item) {
    if (item.type !== 'text') return;
    editingId = item.id;
    editContent = item.content || '';
  }

  function cancelEdit() {
    editingId = null;
    editContent = '';
  }

  function viewFullImage(itemId) {
    viewingImageId = itemId;
  }

  function closeImageViewer() {
    viewingImageId = null;
  }

  async function saveEdit(itemId) {
    if (!editContent.trim()) {
      showActionNotice('内容不能为空');
      return;
    }

    try {
      await clipboardApi.updateItemContent(itemId, editContent);

      // 更新本地列表
      items = items.map((item) =>
        item.id === itemId
          ? {
              ...item,
              content: editContent,
              preview: editContent.length > 100
                ? editContent.substring(0, 100) + '...'
                : editContent,
            }
          : item
      );

      editingId = null;
      editContent = '';
      showActionNotice('保存成功');
    } catch (e) {
      console.error('保存失败:', e);
      showActionNotice('保存失败: ' + e);
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
        <p>剪贴板工作台</p>
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

    <div class="session-card">
      <span class="status-dot"></span>
      <div>
        <strong>本次会话</strong>
        <span>{selectedDay || '全部日期'} · {items.length} 条记录</span>
      </div>
    </div>
  </aside>

  <section class="workspace" aria-label="剪贴板历史">
    <header class="toolbar">
      <div>
        <p class="eyebrow">Clipboard history</p>
        <h2>剪贴板历史</h2>
      </div>

      <div class="toolbar-tools">
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

        <label class="day-field">
          <span>日期</span>
          <select bind:value={selectedDay} on:change={handleDayChange} aria-label="按日期提取剪贴板记录">
            <option value="">全部</option>
            {#each availableDays as day}
              <option value={day.date_key}>{day.date_key}（{day.item_count}）</option>
            {/each}
          </select>
        </label>

        <label class="search-field">
          <Search size={17} aria-hidden="true" />
          <span class="sr-only">搜索剪贴板内容</span>
          <input
            type="search"
            aria-label="搜索剪贴板内容"
            placeholder="搜索文本、代码片段或图片记录"
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
    </header>

    {#if error}
      <div class="notice error" role="alert">{error}</div>
    {/if}

    {#if copySuccess}
      <div class="notice success" role="status">
        <Check size={16} aria-hidden="true" />
        <span>已复制到剪贴板</span>
      </div>
    {/if}

    {#if actionNotice}
      <div class="notice success" role="status">
        <Check size={16} aria-hidden="true" />
        <span>{actionNotice}</span>
      </div>
    {/if}

    <div class="history-panel" data-testid="history-panel" data-scroll="internal">
      {#if loading || isSearching}
        <div class="loading">
          <LoaderCircle size={20} aria-hidden="true" />
          <span>加载中</span>
        </div>
      {:else if visibleItems().length === 0}
        <div class="empty-state">
          <Inbox size={34} aria-hidden="true" />
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
          {#each visibleItems() as item (item.id)}
            <article class="item" class:pinned={item.is_pinned}>
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
                  </div>

                  <div class="item-actions">
                    <button
                      type="button"
                      on:click={() => copyItem(item)}
                      aria-label={`复制 ${itemLabel(item)}`}
                      title="复制"
                    >
                      <Copy size={16} aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      class:active={item.is_pinned}
                      on:click={() => togglePinned(item.id)}
                      aria-label={`置顶 ${itemLabel(item)}`}
                      title="置顶"
                    >
                      <Pin size={16} aria-hidden="true" />
                    </button>
                    {#if item.type === 'image' && item.image_path}
                      <button
                        type="button"
                        on:click={() => pinImageToDesktop(item)}
                        aria-label={`钉到桌面 ${itemLabel(item)}`}
                        title="钉到桌面"
                      >
                        <Pin size={16} aria-hidden="true" />
                      </button>
                    {/if}
                    <button
                      type="button"
                      class:active={item.is_favorite}
                      on:click={() => toggleFavorite(item.id)}
                      aria-label={`收藏 ${itemLabel(item)}`}
                      title="收藏"
                    >
                      <Star size={16} aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      on:click={() => deleteItem(item.id)}
                      aria-label={`删除 ${itemLabel(item)}`}
                      title="删除"
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
                        placeholder="编辑内容"
                        rows="4"
                      ></textarea>
                      <div class="edit-actions">
                        <button
                          type="button"
                          class="btn-save"
                          on:click={() => saveEdit(item.id)}
                        >
                          <Check size={16} aria-hidden="true" />
                          保存
                        </button>
                        <button
                          type="button"
                          class="btn-cancel"
                          on:click={cancelEdit}
                        >
                          <X size={16} aria-hidden="true" />
                          取消
                        </button>
                      </div>
                    </div>
                  {:else}
                    <button
                      type="button"
                      class="text-content"
                      on:click={() => startEdit(item)}
                      on:keydown={(e) => e.key === 'Enter' && startEdit(item)}
                      title="点击编辑"
                    >
                      {item.preview || item.content}
                    </button>
                  {/if}
                {:else if item.type === 'image'}
                  <div class="image-summary">
                    <strong>图片记录</strong>
                    <span>{item.image_path || '等待图片路径'}</span>
                  </div>
                  {#if thumbnailUrls[item.id]}
                    <div class="image-preview" on:click={() => viewFullImage(item.id)} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && viewFullImage(item.id)} title="点击查看原图">
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
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  </section>
  {#if settingsOpen}
    <div class="settings-backdrop" on:click={() => (settingsOpen = false)} aria-hidden="true"></div>
    <div
      class="settings-panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <header class="settings-header">
        <h2 id="settings-title">设置</h2>
        <button type="button" on:click={() => (settingsOpen = false)} aria-label="关闭设置">
          <X size={16} aria-hidden="true" />
        </button>
      </header>

      <div class="settings-content">
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
            on:input={(event) => updateSettingsDraft('max_items', Number(event.currentTarget.value))}
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

        <div class="settings-section">
          <h3>自定义清理</h3>
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
        </div>

        <div class="settings-section">
          <h3>快捷键设置</h3>
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
          <p class="hotkey-hint">
            {#if isRecordingHotkey}
              ⌨️ 正在录制... 请按下组合键（如 Ctrl+Shift+A）
            {:else}
              点击输入框后按下组合键自动录制，例如 Ctrl+Shift+A
            {/if}
          </p>
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

  <!-- 原图查看器 -->
  {#if viewingImageId && imageUrls[viewingImageId]}
    <div class="image-viewer-overlay" on:click={closeImageViewer} role="button" tabindex="0" on:keydown={(e) => e.key === 'Escape' && closeImageViewer()}>
      <div class="image-viewer-content" on:click|stopPropagation role="presentation">
        <button class="image-viewer-close" on:click={closeImageViewer} aria-label="关闭" title="关闭 (ESC)">
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

  .session-card {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: auto;
    padding: 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
  }

  .session-card strong,
  .session-card span {
    display: block;
  }

  .session-card strong {
    color: #ffffff;
    font-size: 0.84rem;
  }

  .session-card span {
    margin-top: 2px;
    color: #94a3b8;
    font-size: 0.74rem;
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
    grid-template-columns: minmax(130px, 0.58fr) minmax(250px, 1fr);
    align-items: end;
    gap: 12px;
  }

  .toolbar-tools {
    display: grid;
    min-width: 0;
    gap: 8px;
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
    color: #475569;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 8px;
    font-size: 0.82rem;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
  }

  .day-field span {
    flex: 0 0 auto;
    font-weight: 700;
  }

  .day-field select {
    min-width: 0;
    width: 100%;
    color: #172033;
    background: transparent;
    border: 0;
    outline: 0;
  }

  .search-field {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
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

  .notice.success {
    color: #166534;
    background: #f0fdf4;
    border: 1px solid #bbf7d0;
  }

  .settings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgba(15, 23, 42, 0.28);
  }

  .settings-panel {
    position: fixed;
    top: 0;
    right: 0;
    z-index: 21;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    width: min(360px, 92vw);
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

  .settings-content {
    display: grid;
    align-content: start;
    gap: 12px;
    min-height: 0;
    padding: 14px;
    overflow: auto;
  }

  .switch-row,
  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    min-height: 42px;
    color: #172033;
    font-size: 0.88rem;
  }

  .switch-row input {
    width: 18px;
    height: 18px;
    accent-color: #2563eb;
  }

  .field-row input {
    width: 96px;
    min-height: 32px;
    padding: 0 8px;
    color: #172033;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 7px;
  }

  .field-row input[type="text"] {
    width: 240px;
  }

  .settings-section {
    display: grid;
    gap: 12px;
    padding-top: 8px;
    border-top: 1px solid #edf1f6;
  }

  .settings-section h3 {
    color: #475569;
    font-size: 0.82rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
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

  .loading,
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

  .loading :global(svg) {
    animation: spin 900ms linear infinite;
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
    padding: 11px 14px;
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
    gap: 10px;
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
    min-height: 22px;
    padding: 2px 7px;
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
    margin-top: 9px;
    color: #172033;
    font: inherit;
    font-size: 0.92rem;
    line-height: 1.45;
    text-align: left;
    word-break: break-word;
    cursor: pointer;
    background: transparent;
    border: 0;
    transition: background 0.15s;
    padding: 4px;
    border-radius: 4px;
  }

  .text-content:hover {
    background: #f1f5f9;
  }

  .edit-area {
    margin-top: 9px;
  }

  .edit-area textarea {
    width: 100%;
    min-height: 100px;
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
    margin-top: 9px;
    max-height: 180px;
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
    max-width: 100%;
    max-height: 180px;
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
    gap: 6px;
  }

  .item-actions button {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    color: #475569;
    background: #ffffff;
    border: 1px solid #d9e0ea;
    border-radius: 7px;
    cursor: pointer;
  }

  .item-actions button:hover,
  .item-actions button.active {
    color: #1d4ed8;
    background: #eff6ff;
    border-color: #bfdbfe;
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
      grid-template-columns: minmax(150px, 1fr) auto;
      grid-template-areas:
        'brand session'
        'filters filters';
      align-items: center;
      gap: 8px;
      flex: 0 0 auto;
      padding: 10px 14px;
    }

    .brand {
      grid-area: brand;
      min-width: 160px;
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

    .session-card {
      grid-area: session;
      margin-top: 0;
      justify-self: end;
      min-width: 112px;
      padding: 7px 9px;
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
</style>
