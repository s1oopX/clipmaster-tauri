<script>
  import { onMount } from 'svelte';
  import { clipboardApi, sessionApi } from './lib/api.js';

  let items = [];
  let currentSession = null;
  let loading = false;
  let error = null;

  onMount(async () => {
    console.log('ClipMaster Tauri 启动成功！');

    try {
      // 获取当前会话
      currentSession = await sessionApi.getCurrentSession();
      console.log('当前会话:', currentSession);

      // 获取剪贴板列表
      await loadItems();

      // 监听新记录
      clipboardApi.onNewItem((item) => {
        console.log('新剪贴板记录:', item);
        items = [item, ...items];
      });
    } catch (e) {
      console.error('初始化失败:', e);
      error = e.toString();
    }
  });

  async function loadItems() {
    loading = true;
    try {
      items = await clipboardApi.getItems(50, 0);
      console.log('已加载记录:', items.length);
    } catch (e) {
      console.error('加载记录失败:', e);
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  async function deleteItem(itemId) {
    try {
      await clipboardApi.deleteItem(itemId);
      items = items.filter(item => item.id !== itemId);
    } catch (e) {
      console.error('删除失败:', e);
      alert('删除失败: ' + e);
    }
  }

  async function toggleFavorite(itemId) {
    try {
      const isFavorite = await clipboardApi.toggleFavorite(itemId);
      items = items.map(item =>
        item.id === itemId ? { ...item, is_favorite: isFavorite } : item
      );
    } catch (e) {
      console.error('切换收藏失败:', e);
    }
  }

  function formatTime(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;

    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;

    return date.toLocaleString('zh-CN');
  }

  function getTypeIcon(type) {
    switch (type) {
      case 'text': return '📝';
      case 'image': return '🖼️';
      case 'file': return '📁';
      default: return '📋';
    }
  }
</script>

<main>
  <div class="container">
    <header>
      <h1>📋 ClipMaster</h1>
      <p class="subtitle">Tauri + Rust + Svelte</p>

      {#if currentSession}
        <div class="session-info">
          <div class="status-dot"></div>
          <span>本次会话 · {items.length} 条记录</span>
        </div>
      {/if}
    </header>

    {#if error}
      <div class="error-box">
        ⚠️ {error}
      </div>
    {/if}

    <div class="content">
      {#if loading}
        <div class="loading">加载中...</div>
      {:else if items.length === 0}
        <div class="empty">
          <p>📋</p>
          <p>暂无剪贴板记录</p>
          <p class="hint">复制一些内容试试吧！</p>
        </div>
      {:else}
        <div class="items-list">
          {#each items as item (item.id)}
            <div class="item" class:pinned={item.is_pinned}>
              <div class="item-header">
                <span class="type-icon">{getTypeIcon(item.type)}</span>
                <span class="time">{formatTime(item.timestamp)}</span>
                {#if item.is_pinned}
                  <span class="badge">📌 置顶</span>
                {/if}
                {#if item.is_favorite}
                  <span class="badge">⭐ 收藏</span>
                {/if}
              </div>

              <div class="item-content">
                {#if item.type === 'text'}
                  <p class="text-content">{item.preview || item.content}</p>
                {:else if item.type === 'image'}
                  <p class="image-placeholder">🖼️ 图片</p>
                {/if}
              </div>

              <div class="item-actions">
                <button
                  class="btn-icon"
                  class:active={item.is_favorite}
                  on:click={() => toggleFavorite(item.id)}
                  title="收藏"
                >
                  {item.is_favorite ? '⭐' : '☆'}
                </button>
                <button
                  class="btn-icon"
                  on:click={() => deleteItem(item.id)}
                  title="删除"
                >
                  🗑️
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #f5f5f5;
  }

  main {
    width: 100%;
    min-height: 100vh;
    padding: 20px;
  }

  .container {
    max-width: 800px;
    margin: 0 auto;
  }

  header {
    text-align: center;
    margin-bottom: 30px;
    padding: 20px;
    background: white;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }

  h1 {
    margin: 0 0 10px 0;
    font-size: 2rem;
    color: #333;
  }

  .subtitle {
    margin: 0;
    color: #666;
    font-size: 0.9rem;
  }

  .session-info {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-top: 15px;
    padding: 8px 16px;
    background: #e8f5e9;
    border-radius: 20px;
    font-size: 0.85rem;
    color: #2e7d32;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    background: #4caf50;
    border-radius: 50%;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .error-box {
    padding: 15px;
    background: #ffebee;
    border: 1px solid #f44336;
    border-radius: 8px;
    color: #c62828;
    margin-bottom: 20px;
  }

  .content {
    background: white;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    min-height: 400px;
  }

  .loading, .empty {
    text-align: center;
    padding: 60px 20px;
    color: #999;
  }

  .empty p:first-child {
    font-size: 3rem;
    margin: 0 0 10px 0;
  }

  .empty .hint {
    font-size: 0.85rem;
    margin-top: 10px;
  }

  .items-list {
    padding: 15px;
  }

  .item {
    padding: 15px;
    margin-bottom: 10px;
    background: #fafafa;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    transition: all 0.2s;
  }

  .item:hover {
    background: #f5f5f5;
    border-color: #2196f3;
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  }

  .item.pinned {
    background: #fff9c4;
    border-color: #fbc02d;
  }

  .item-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
    font-size: 0.85rem;
    color: #666;
  }

  .type-icon {
    font-size: 1.2rem;
  }

  .time {
    flex: 1;
  }

  .badge {
    padding: 2px 8px;
    background: #e3f2fd;
    border-radius: 12px;
    font-size: 0.75rem;
    color: #1976d2;
  }

  .item-content {
    margin-bottom: 10px;
  }

  .text-content {
    margin: 0;
    color: #333;
    line-height: 1.6;
    word-break: break-word;
  }

  .image-placeholder {
    margin: 0;
    padding: 20px;
    text-align: center;
    background: #e3f2fd;
    border-radius: 6px;
    color: #1976d2;
  }

  .item-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .btn-icon {
    padding: 6px 12px;
    background: white;
    border: 1px solid #ddd;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: #f5f5f5;
    border-color: #999;
    transform: scale(1.1);
  }

  .btn-icon.active {
    background: #fff9c4;
    border-color: #fbc02d;
  }
</style>
