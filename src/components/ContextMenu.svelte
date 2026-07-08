<script>
  import { Copy, ExternalLink, FileText } from '@lucide/svelte';
  import { effectiveItemType } from '../lib/clipboard-ui.js';

  export let activeContextItem = null;
  export let contextMenu = { open: false, x: 0, y: 0, itemId: null };
  export let onAddAnnotation = () => {};
  export let onCopy = () => {};
  export let onEditContent = () => {};
  export let onOpenLink = () => {};
  export let runContextAction = (action) => action();

  function positionContextMenu(node, menu) {
    function update(nextMenu) {
      node.style.left = `${nextMenu.x}px`;
      node.style.top = `${nextMenu.y}px`;
    }

    update(menu);

    return { update };
  }
</script>

{#if contextMenu.open && activeContextItem}
  <div
    class="context-menu"
    role="menu"
    tabindex="-1"
    use:positionContextMenu={contextMenu}
  >
    <button
      type="button"
      role="menuitem"
      on:click={() => runContextAction(() => onCopy(activeContextItem))}
    >
      <Copy size={15} aria-hidden="true" />
      复制
    </button>
    {#if effectiveItemType(activeContextItem) === 'link'}
      <button
        type="button"
        role="menuitem"
        on:click={() => runContextAction(() => onOpenLink(activeContextItem))}
      >
        <ExternalLink size={15} aria-hidden="true" />
        打开链接
      </button>
    {/if}
    {#if effectiveItemType(activeContextItem) === 'text'}
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
