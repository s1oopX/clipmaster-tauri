import {
  activeFilterQuery as filterQueryFor,
  mergeItems,
  pageSize as configuredPageSize,
} from './app-helpers.js';

export function createRecordsController({
  clipboardApi,
  searchApi,
  defaultSettings,
  getActiveFilter,
  getAppSettings,
  getItems,
  setItems,
  getSearchQuery,
  setSearchQuery,
  getSelectedDay,
  setSelectedDay,
  getLoading,
  setLoading,
  getLoadingMore,
  setLoadingMore,
  getIsSearching,
  setIsSearching,
  getHasMoreRecords,
  setHasMoreRecords,
  setError,
  clearActionError,
  showActionError,
  pruneImageUrls,
  reconcileTransientItemState,
  loadImageUrls,
}) {
  let recordsRequestId = 0;
  let searchTimer = null;

  function pageSize() {
    return configuredPageSize(getAppSettings(), defaultSettings);
  }

  function activeSearchDateKey() {
    // 未选日期时返回 null = 搜索全部历史（后端 FTS 支撑跨天检索）
    return getSelectedDay() || null;
  }

  function activeFilterQuery() {
    return filterQueryFor(getActiveFilter());
  }

  async function loadItems(day = getSelectedDay(), { append = false } = {}) {
    const requestId = ++recordsRequestId;
    setIsSearching(false);
    if (append) {
      setLoadingMore(true);
    } else {
      setLoading(true);
      setHasMoreRecords(false);
    }

    try {
      const offset = append ? getItems().length : 0;
      const limit = pageSize();
      const filter = activeFilterQuery();
      const hasFilter = Object.keys(filter).length > 0;
      const nextItems = day
        ? hasFilter
          ? await clipboardApi.getItemsByDay(day, limit, offset, filter)
          : await clipboardApi.getItemsByDay(day, limit, offset)
        : hasFilter
          ? await clipboardApi.getItems(limit, offset, filter)
          : await clipboardApi.getItems(limit, offset);

      if (requestId !== recordsRequestId) return;

      const visibleItems = append ? mergeItems(getItems(), nextItems) : nextItems;
      setItems(visibleItems);
      setHasMoreRecords(nextItems.length === limit);
      setError(null);
      clearActionError();
      pruneImageUrls(visibleItems);
      reconcileTransientItemState(visibleItems);
      void loadImageUrls();
    } catch (e) {
      if (requestId !== recordsRequestId) return;

      console.error('加载记录失败:', e);
      setError(e.toString());
    } finally {
      if (requestId === recordsRequestId) {
        setLoading(false);
        setLoadingMore(false);
      }
    }
  }

  async function handleSearch({ append = false } = {}) {
    const query = getSearchQuery().trim();

    if (!query) {
      await loadItems(getSelectedDay(), { append });
      return;
    }

    const requestId = ++recordsRequestId;
    if (append) {
      setLoadingMore(true);
    } else {
      setLoading(false);
      setIsSearching(true);
      setHasMoreRecords(false);
    }

    try {
      const dateKey = activeSearchDateKey();
      const limit = pageSize();
      const offset = append ? getItems().length : 0;
      const filter = activeFilterQuery();
      const nextItems = Object.keys(filter).length > 0
        ? await searchApi.searchItems(query, null, limit, dateKey, offset, filter)
        : await searchApi.searchItems(query, null, limit, dateKey, offset);

      if (requestId !== recordsRequestId) return;

      const visibleItems = append ? mergeItems(getItems(), nextItems) : nextItems;
      setItems(visibleItems);
      setHasMoreRecords(nextItems.length === limit);
      setError(null);
      clearActionError();
      pruneImageUrls(visibleItems);
      reconcileTransientItemState(visibleItems);
      void loadImageUrls();
    } catch (e) {
      if (requestId !== recordsRequestId) return;

      console.error('搜索失败:', e);
      setError(null);
      showActionError('搜索失败: ' + e);
    } finally {
      if (requestId === recordsRequestId) {
        setIsSearching(false);
        setLoadingMore(false);
      }
    }
  }

  async function refreshVisibleRecords() {
    if (getSearchQuery().trim()) {
      await handleSearch();
    } else {
      await loadItems();
    }
  }

  async function selectDay(day) {
    setSelectedDay(day);
    setSearchQuery('');
    await loadItems(day);
  }

  async function clearDayFilter() {
    await selectDay('');
  }

  function clearSearch() {
    if (searchTimer) clearTimeout(searchTimer);
    setSearchQuery('');
    void loadItems();
  }

  function queueSearch() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void handleSearch();
    }, 200);
  }

  async function loadMoreRecords() {
    if (getLoading() || getLoadingMore() || getIsSearching() || !getHasMoreRecords()) return;

    if (getSearchQuery().trim()) {
      await handleSearch({ append: true });
    } else {
      await loadItems(getSelectedDay(), { append: true });
    }
  }

  function invalidateRequests() {
    recordsRequestId++;
  }

  function dispose() {
    if (searchTimer) clearTimeout(searchTimer);
  }

  return {
    activeSearchDateKey,
    clearDayFilter,
    clearSearch,
    dispose,
    handleSearch,
    invalidateRequests,
    loadItems,
    loadMoreRecords,
    queueSearch,
    refreshVisibleRecords,
    selectDay,
  };
}
