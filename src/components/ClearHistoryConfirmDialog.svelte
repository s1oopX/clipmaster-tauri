<script>
  import { Trash2 } from '@lucide/svelte';

  export let clearHistoryLoading;
  export let onCancel;
  export let onConfirm;
</script>

<button
  type="button"
  class="confirm-backdrop"
  aria-label="取消清空历史确认"
  on:click={onCancel}
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
      on:click={onCancel}
      disabled={clearHistoryLoading}
    >
      取消
    </button>
    <button
      type="button"
      class="danger-button"
      on:click={onConfirm}
      disabled={clearHistoryLoading}
    >
      {clearHistoryLoading ? '清空中' : '确认清空'}
    </button>
  </footer>
</div>
