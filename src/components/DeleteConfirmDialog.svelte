<script>
  import { Trash2 } from '@lucide/svelte';

  export let deleteCandidate = null;
  export let deleteConfirmLoading = false;
  export let deleteReasonLabel = () => '';
  export let itemLabel = () => '剪贴板记录';
  export let onCancel = () => {};
  export let onConfirm = () => {};
</script>

{#if deleteCandidate}
  <button
    type="button"
    class="confirm-backdrop"
    aria-label="取消删除确认"
    on:click={onCancel}
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
        on:click={onCancel}
        disabled={deleteConfirmLoading}
      >
        取消
      </button>
      <button
        type="button"
        class="danger-button"
        on:click={onConfirm}
        disabled={deleteConfirmLoading}
      >
        {deleteConfirmLoading ? '删除中' : '确认删除'}
      </button>
    </footer>
  </div>
{/if}
