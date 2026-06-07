<script>
  import { Check, X } from '@lucide/svelte';

  export let actionError = '';
  export let actionNotice = '';
  export let copySuccess = false;
</script>

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
