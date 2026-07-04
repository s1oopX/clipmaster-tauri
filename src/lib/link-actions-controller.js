import { isActivationKey } from './clipboard-ui.js';

export function createLinkActionsController({ toolApi, showActionError }) {
  async function openLinkUrl(url) {
    if (!url) return;

    try {
      await toolApi.openExternalUrl(url);
    } catch (e) {
      console.error('打开链接失败:', e);
      showActionError('打开链接失败: ' + e);
    }
  }

  async function openRecordLink(event, item) {
    if (!event.ctrlKey && !event.metaKey) {
      return;
    }

    event.preventDefault();
    await openLinkUrl(item.content);
  }

  function handleRecordLinkKeyDown(event, item) {
    if (!isActivationKey(event)) return;
    event.preventDefault();
    void openLinkUrl(item.content);
  }

  return {
    handleRecordLinkKeyDown,
    openLinkUrl,
    openRecordLink,
  };
}
