<script>
  import ClipboardHistoryPanel from './ClipboardHistoryPanel.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import DeleteConfirmDialog from './DeleteConfirmDialog.svelte';
  import HistoryToolbar from './HistoryToolbar.svelte';
  import ImageViewer from './ImageViewer.svelte';
  import SettingsController from './SettingsController.svelte';
  import Sidebar from './Sidebar.svelte';
  import ToastStack from './ToastStack.svelte';

  export let activeContextItem = null;
  export let activeFilter = 'all';
  export let actionError = '';
  export let actionNotice = '';
  export let annotationDraft = '';
  export let annotationEditingId = null;
  export let appSettings = {};
  export let availableDays = [];
  export let contextMenu = {};
  export let copySuccess = false;
  export let datePicker;
  export let deleteCandidate = null;
  export let deleteConfirmLoading = false;
  export let deleteReasonLabel;
  export let editContent = '';
  export let editingId = null;
  export let error = null;
  export let filteredItems = [];
  export let filters = [];
  export let hasMoreRecords = false;
  export let imagePreviewErrors = {};
  export let imageUrls = {};
  export let isSearching = false;
  export let itemLabel;
  export let loading = false;
  export let loadingMore = false;
  export let monitorToggleSaving = false;
  export let recordsScope = '';
  export let searchInput = null;
  export let searchQuery = '';
  export let selectedDay = '';
  export let settingsOpen = false;
  export let settingsSaving = false;
  export let thumbnailUrls = {};
  export let toolLoading = null;
  export let viewingImageId = null;

  export let onCancelAnnotationEdit = () => {};
  export let onCancelContentEdit = () => {};
  export let onCancelDeleteConfirmation = () => {};
  export let onClearDayFilter = () => {};
  export let onClearSearch = () => {};
  export let onCloseImageViewer = () => {};
  export let onCloseImageViewerFromKeyboard = () => {};
  export let onConfirmDeleteCandidate = () => {};
  export let onCopyItem = () => {};
  export let onFallbackToOriginalPreview = () => {};
  export let onHistoryCleared = () => {};
  export let onLoadMoreRecords = () => {};
  export let onOpenContextMenu = () => {};
  export let onOpenLink = () => {};
  export let onOpenRecordLink = () => {};
  export let onOpenSettings = () => {};
  export let onPinImageToDesktop = () => {};
  export let onPinNewestImage = () => {};
  export let onQueueSearch = () => {};
  export let onRecordLinkKeyDown = () => {};
  export let onRefreshSettingsRecords = () => {};
  export let onRequestDeleteItem = () => {};
  export let onSaveAnnotation = () => {};
  export let onSaveContentEdit = () => {};
  export let onSelectDay = () => {};
  export let onSelectFilter = () => {};
  export let onStartAnnotationEdit = () => {};
  export let onStartContentEdit = () => {};
  export let onStartScreenshot = () => {};
  export let onToggleFavorite = () => {};
  export let onToggleMonitoring = () => {};
  export let onTogglePinned = () => {};
  export let onViewFullImage = () => {};
  export let runContextAction = () => {};
  export let showActionError = () => {};
  export let showActionNotice = () => {};
</script>

<main
  class="app-shell"
  data-testid="app-shell"
  data-layout="compact-ready"
  data-density="tool"
  data-reference="figma-utility-grid"
>
  <Sidebar {activeFilter} {filters} onFilterChange={onSelectFilter} />

  <section class="workspace" aria-label="剪贴板历史">
    <HistoryToolbar
      bind:searchInput
      bind:searchQuery
      {appSettings}
      {availableDays}
      {datePicker}
      filteredCount={filteredItems.length}
      {monitorToggleSaving}
      {recordsScope}
      {selectedDay}
      {settingsSaving}
      {toolLoading}
      onClearDayFilter={onClearDayFilter}
      onClearSearch={onClearSearch}
      onOpenSettings={onOpenSettings}
      onPinNewestImage={onPinNewestImage}
      onQueueSearch={onQueueSearch}
      onSelectDay={onSelectDay}
      onStartScreenshot={onStartScreenshot}
      onToggleMonitoring={onToggleMonitoring}
    />

    {#if error}
      <div class="notice error" role="alert">{error}</div>
    {/if}

    <ClipboardHistoryPanel
      bind:annotationDraft
      bind:editContent
      {annotationEditingId}
      {editingId}
      {filteredItems}
      {hasMoreRecords}
      {imagePreviewErrors}
      {isSearching}
      {loading}
      {loadingMore}
      {searchQuery}
      {thumbnailUrls}
      onCancelAnnotationEdit={onCancelAnnotationEdit}
      onCancelContentEdit={onCancelContentEdit}
      onCopyItem={onCopyItem}
      onFallbackToOriginalPreview={onFallbackToOriginalPreview}
      onLoadMoreRecords={onLoadMoreRecords}
      onOpenContextMenu={onOpenContextMenu}
      onOpenLink={onOpenLink}
      onOpenRecordLink={onOpenRecordLink}
      onPinImageToDesktop={onPinImageToDesktop}
      onRecordLinkKeyDown={onRecordLinkKeyDown}
      onRequestDeleteItem={onRequestDeleteItem}
      onSaveAnnotation={onSaveAnnotation}
      onSaveContentEdit={onSaveContentEdit}
      onStartAnnotationEdit={onStartAnnotationEdit}
      onToggleFavorite={onToggleFavorite}
      onTogglePinned={onTogglePinned}
      onViewFullImage={onViewFullImage}
    />

    <ToastStack {copySuccess} {actionNotice} {actionError} />
  </section>

  <ContextMenu
    {activeContextItem}
    {contextMenu}
    {runContextAction}
    onAddAnnotation={onStartAnnotationEdit}
    onCopy={onCopyItem}
    onEditContent={onStartContentEdit}
    onOpenLink={(item) => onOpenLink(item.content)}
  />

  {#if settingsOpen}
    <SettingsController
      bind:appSettings
      bind:selectedDay
      bind:settingsSaving
      onClose={() => (settingsOpen = false)}
      onHistoryCleared={onHistoryCleared}
      onRefreshRecords={onRefreshSettingsRecords}
      {showActionError}
      {showActionNotice}
    />
  {/if}

  <DeleteConfirmDialog
    {deleteCandidate}
    {deleteConfirmLoading}
    {deleteReasonLabel}
    {itemLabel}
    onCancel={onCancelDeleteConfirmation}
    onConfirm={onConfirmDeleteCandidate}
  />

  <ImageViewer
    {imageUrls}
    {viewingImageId}
    onClose={onCloseImageViewer}
    onKeyboardClose={onCloseImageViewerFromKeyboard}
  />
</main>
