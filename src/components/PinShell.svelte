<script>
  import { X } from '@lucide/svelte';

  export let pinImagePath = '';
  export let pinImageUrl = '';
  export let onClose = () => {};
</script>

<main class="pin-shell" data-testid="pin-shell">
  <header class="pin-toolbar" data-tauri-drag-region>
    <span>ClipMaster 贴图</span>
    <button type="button" on:click={onClose} aria-label="关闭贴图">
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
