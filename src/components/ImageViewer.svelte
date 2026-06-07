<script>
  import { X } from '@lucide/svelte';

  export let imageUrls = {};
  export let onClose = () => {};
  export let onKeyboardClose = () => {};
  export let viewingImageId = null;
</script>

{#if viewingImageId && imageUrls[viewingImageId]}
  <div
    class="image-viewer-overlay"
    on:click={onClose}
    role="button"
    tabindex="0"
    on:keydown={onKeyboardClose}
  >
    <div class="image-viewer-content" on:click|stopPropagation role="presentation">
      <button class="image-viewer-close" on:click={onClose} aria-label="关闭">
        <X size={24} aria-hidden="true" />
      </button>
      <img
        src={imageUrls[viewingImageId]}
        alt="原图"
        on:error={() => {
          console.error('原图加载失败');
          onClose();
        }}
      />
    </div>
  </div>
{/if}
