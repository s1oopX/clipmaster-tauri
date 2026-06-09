<script>
  import { Clipboard, Heart, Image as ImageIcon, Link, List } from '@lucide/svelte';

  export let activeFilter = 'all';
  export let filters = [];
  export let onFilterChange = () => {};
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="brand-mark">
      <Clipboard size={20} aria-hidden="true" />
    </div>
    <div>
      <h1>ClipMaster</h1>
      <p>快速剪存与回放</p>
    </div>
  </div>

  <nav class="filter-nav" aria-label="剪贴板筛选">
    {#each filters as filter}
      <button
        class="filter-button"
        class:active={activeFilter === filter.id}
        on:click={() => onFilterChange(filter.id)}
        type="button"
      >
        {#if filter.id === 'all'}
          <List size={16} aria-hidden="true" />
        {:else if filter.id === 'favorite'}
          <Heart size={16} aria-hidden="true" />
        {:else if filter.id === 'link'}
          <Link size={16} aria-hidden="true" />
        {:else}
          <ImageIcon size={16} aria-hidden="true" />
        {/if}
        <span>{filter.label}</span>
      </button>
    {/each}
  </nav>
</aside>
