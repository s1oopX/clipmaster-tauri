<script>
  import { Copy, FileText } from '@lucide/svelte';

  export let activeContextItem = null;
  export let contextMenu = { open: false, x: 0, y: 0, itemId: null };
  export let onAddAnnotation = () => {};
  export let onCopy = () => {};
  export let onEditContent = () => {};
  export let runContextAction = (action) => action();
</script>

{#if contextMenu.open && activeContextItem}
  <div
    class="context-menu"
    role="menu"
    tabindex="-1"
    style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px;`}
  >
    <button
      type="button"
      role="menuitem"
      on:click={() => runContextAction(() => onCopy(activeContextItem))}
    >
      <Copy size={15} aria-hidden="true" />
      复制
    </button>
    {#if activeContextItem.type === 'text'}
      <button
        type="button"
        role="menuitem"
        on:click={() => runContextAction(() => onEditContent(activeContextItem))}
      >
        <FileText size={15} aria-hidden="true" />
        编辑原文
      </button>
    {/if}
    <button
      type="button"
      role="menuitem"
      on:click={() => runContextAction(() => onAddAnnotation(activeContextItem))}
    >
      <FileText size={15} aria-hidden="true" />
      {activeContextItem.annotation ? '编辑标注' : '添加标注'}
    </button>
  </div>
{/if}
