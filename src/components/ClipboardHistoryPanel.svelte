<script>
  import {
    Check,
    Copy,
    ExternalLink,
    FileText,
    Image as ImageIcon,
    Inbox,
    LoaderCircle,
    Pin,
    Star,
    Trash2,
    X,
  } from '@lucide/svelte';
  import {
    effectiveItemType,
    formatTime,
    itemLabel,
    linkDisplayLabel,
    runKeyboardAction,
  } from '../lib/clipboard-ui.js';

  export let annotationDraft = '';
  export let annotationEditingId = null;
  export let editContent = '';
  export let editingId = null;
  export let filteredItems = [];
  export let hasMoreRecords = false;
  export let imagePreviewErrors = {};
  export let isSearching = false;
  export let loading = false;
  export let loadingMore = false;
  export let searchQuery = '';
  export let thumbnailUrls = {};

  export let onCancelAnnotationEdit = () => {};
  export let onCancelContentEdit = () => {};
  export let onCopyItem = () => {};
  export let onFallbackToOriginalPreview = () => {};
  export let onLoadMoreRecords = () => {};
  export let onOpenContextMenu = () => {};
  export let onOpenLink = () => {};
  export let onOpenRecordLink = () => {};
  export let onPinImageToDesktop = () => {};
  export let onRecordLinkKeyDown = () => {};
  export let onRequestDeleteItem = () => {};
  export let onSaveAnnotation = () => {};
  export let onSaveContentEdit = () => {};
  export let onStartAnnotationEdit = () => {};
  export let onToggleFavorite = () => {};
  export let onTogglePinned = () => {};
  export let onViewFullImage = () => {};
</script>

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
          on:contextmenu={(event) => onOpenContextMenu(event, item)}
        >
          <div class="item-main">
            <div class="item-row">
              <div class="item-meta">
                <span class="type-pill">
                  {#if item.type === 'image'}
                    <ImageIcon size={14} aria-hidden="true" />
                    图片
                  {:else if effectiveItemType(item) === 'link'}
                    <ExternalLink size={14} aria-hidden="true" />
                    链接
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
                  on:click={() => onCopyItem(item)}
                  aria-label={`复制 ${itemLabel(item)}`}
                >
                  <Copy size={16} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  class="item-action secondary-action"
                  class:active={item.is_pinned}
                  on:click={() => onTogglePinned(item.id)}
                  aria-label={`置顶 ${itemLabel(item)}`}
                >
                  <Pin size={16} aria-hidden="true" />
                </button>
                {#if item.type === 'image' && item.image_path}
                  <button
                    type="button"
                    class="item-action secondary-action"
                    on:click={() => onPinImageToDesktop(item)}
                    aria-label={`钉到桌面 ${itemLabel(item)}`}
                  >
                    <Pin size={16} aria-hidden="true" />
                  </button>
                {/if}
                {#if effectiveItemType(item) === 'link' && item.content}
                  <button
                    type="button"
                    class="item-action secondary-action"
                    on:click={() => onOpenLink(item.content)}
                    aria-label={`打开 ${itemLabel(item)}`}
                  >
                    <ExternalLink size={16} aria-hidden="true" />
                  </button>
                {/if}
                <button
                  type="button"
                  class="item-action secondary-action"
                  class:active={annotationEditingId === item.id}
                  on:click={() => onStartAnnotationEdit(item)}
                  aria-label={`标注 ${itemLabel(item)}`}
                >
                  <FileText size={16} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  class="item-action primary-action"
                  class:active={item.is_favorite}
                  on:click={() => onToggleFavorite(item.id)}
                  aria-label={`收藏 ${itemLabel(item)}`}
                >
                  <Star size={16} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  class="item-action secondary-action danger-action"
                  on:click={() => onRequestDeleteItem(item)}
                  aria-label={`删除 ${itemLabel(item)}`}
                >
                  <Trash2 size={16} aria-hidden="true" />
                </button>
              </div>
            </div>

            {#if effectiveItemType(item) === 'link'}
              <div
                class="link-content"
                role="link"
                tabindex="0"
                on:click={(event) => onOpenRecordLink(event, item)}
                on:keydown={(event) => onRecordLinkKeyDown(event, item)}
              >
                <div class="link-copy">
                  <ExternalLink size={15} aria-hidden="true" />
                  <span>{linkDisplayLabel(item.content || item.preview)}</span>
                </div>
                <p class="sr-only">Ctrl/Command+左键打开，Enter 直接打开。复制按钮会复制原始链接。</p>
              </div>
            {:else if item.type === 'text'}
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
                      on:click={() => onSaveContentEdit(item.id)}
                    >
                      <Check size={16} aria-hidden="true" />
                      保存原文
                    </button>
                    <button
                      type="button"
                      class="btn-cancel"
                      on:click={onCancelContentEdit}
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
                    onCopyItem(item);
                  }}
                  on:keydown={(event) => {
                    runKeyboardAction(event, () => onCopyItem(item));
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
                  on:click={() => onViewFullImage(item.id)}
                  role="button"
                  tabindex="0"
                  on:keydown={(event) => runKeyboardAction(event, () => onViewFullImage(item.id))}
                >
                  <img
                    src={thumbnailUrls[item.id]}
                    alt="剪贴板图片缩略图"
                    loading="lazy"
                    decoding="async"
                    on:error={() => {
                      console.error('缩略图加载失败:', item.thumbnail_path);
                      void onFallbackToOriginalPreview(item);
                    }}
                  />
                </div>
              {:else if imagePreviewErrors[item.id]}
                <div class="image-loading">图片预览不可用</div>
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
                    on:click={() => onSaveAnnotation(item.id)}
                  >
                    <Check size={16} aria-hidden="true" />
                    保存标注
                  </button>
                  <button
                    type="button"
                    class="btn-cancel"
                    on:click={onCancelAnnotationEdit}
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
      {#if hasMoreRecords}
        <div class="load-more-row">
          <button type="button" on:click={onLoadMoreRecords} disabled={loadingMore}>
            {#if loadingMore}
              <LoaderCircle size={15} aria-hidden="true" />
              加载中
            {:else}
              加载更多
            {/if}
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>
