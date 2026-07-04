<script>
  import {
    Camera,
    CalendarDays,
    LoaderCircle,
    Pause,
    Pin,
    Play,
    Search,
    Settings,
    X,
  } from '@lucide/svelte';

  export let appSettings;
  export let availableDays;
  export let datePicker;
  export let filteredCount;
  export let monitorToggleSaving;
  export let recordsScope;
  export let searchInput = null;
  export let searchQuery;
  export let selectedDay;
  export let settingsSaving;
  export let toolLoading;
  export let onClearDayFilter;
  export let onClearSearch;
  export let onOpenSettings;
  export let onPinNewestImage;
  export let onQueueSearch;
  export let onSelectDay;
  export let onStartScreenshot;
  export let onToggleMonitoring;
</script>

<header class="toolbar">
  <div class="toolbar-title">
    <div class="toolbar-heading">
      <h2>剪贴板历史</h2>
      <p class="toolbar-context" aria-label="当前范围">
        <span class="status-dot"></span>
        {recordsScope} · 已加载 {filteredCount} 条
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
          on:click={() => onToggleMonitoring(!appSettings.clipboard_monitor_enabled)}
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
          on:click={onStartScreenshot}
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
          on:click={onPinNewestImage}
          disabled={toolLoading === 'pin'}
        >
          <Pin size={15} aria-hidden="true" />
          <span>钉住</span>
        </button>

        <button type="button" class="icon-tool" on:click={onOpenSettings} aria-label="设置">
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
          <button type="button" class="clear-date" on:click={onClearDayFilter} aria-label="清除日期筛选">
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
              on:click={() => onSelectDay(day.date_key)}
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
          on:input={onQueueSearch}
        />
        {#if searchQuery}
          <button type="button" class="clear-search" on:click={onClearSearch} aria-label="清除搜索">
            <X size={15} aria-hidden="true" />
          </button>
        {/if}
      </label>
    </div>
  </div>
</header>
