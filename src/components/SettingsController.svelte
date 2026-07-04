<script>
  import { onDestroy } from 'svelte';
  import { settingsApi, toolApi } from '../lib/api.js';
  import ClearHistoryConfirmDialog from './ClearHistoryConfirmDialog.svelte';
  import SettingsPanel from './SettingsPanel.svelte';
  import {
    appVersion,
    defaultSettings,
    githubIssuesUrl,
    githubProfileUrl,
    languageOptions,
    settingsViews,
    timeZoneOptions,
  } from '../lib/app-config.js';
  import {
    hotkeyFromKeyboardEvent,
    numberSettingValue,
  } from '../lib/app-helpers.js';

  export let appSettings;
  export let selectedDay = '';
  export let settingsSaving = false;
  export let onClose = () => {};
  export let onRefreshRecords = async () => {};
  export let onHistoryCleared = async () => {};
  export let showActionNotice = () => {};
  export let showActionError = () => {};

  let activeSettingsView = 'basic';
  let settingsDraft = { ...appSettings };
  let cleanupLoading = false;
  let cleanupPlan = null;
  let clearHistoryConfirmOpen = false;
  let clearHistoryLoading = false;
  let portCheckLoading = false;
  let portCheckResult = null;
  let pendingRestartPort = null;
  let restartingApp = false;
  let appDataDir = '';
  let appDataDirError = '';
  let isRecordingHotkey = false;
  let recordingHotkeyField = null;
  let hotkeyRecordingMessage = '';
  let recordingHotkeyTimeout = null;

  $: currentDraftPort = numberSettingValue(
    settingsDraft.dev_server_port,
    defaultSettings.dev_server_port
  );
  $: settingsPortChanged = currentDraftPort
    !== (appSettings.dev_server_port || defaultSettings.dev_server_port);

  loadAppDataDir();

  onDestroy(() => {
    if (recordingHotkeyTimeout) {
      clearTimeout(recordingHotkeyTimeout);
    }
  });

  async function loadAppDataDir() {
    try {
      appDataDir = await settingsApi.getAppDataDir();
      appDataDirError = '';
    } catch (e) {
      console.error('读取数据目录失败:', e);
      appDataDir = '';
      appDataDirError = e.toString();
    }
  }

  function updateSettingsDraft(key, value) {
    if (key === 'dev_server_port') {
      portCheckResult = null;
      pendingRestartPort = null;
    }

    settingsDraft = {
      ...settingsDraft,
      [key]: value,
    };
  }

  async function checkDevServerPort() {
    portCheckLoading = true;
    portCheckResult = null;

    try {
      portCheckResult = await settingsApi.checkDevServerPort(currentDraftPort);
    } catch (e) {
      console.error('检查端口失败:', e);
      showActionError('检查端口失败: ' + e);
    } finally {
      portCheckLoading = false;
    }
  }

  function applySuggestedPort(port) {
    updateSettingsDraft('dev_server_port', port);
    portCheckResult = {
      port,
      available: true,
      suggested_port: null,
      message: `端口 ${port} 可用`,
    };
  }

  async function restartApplication() {
    restartingApp = true;

    try {
      await settingsApi.restartApp();
    } catch (e) {
      console.error('重启应用失败:', e);
      showActionError('重启应用失败: ' + e);
      restartingApp = false;
    }
  }

  async function saveSettings() {
    settingsSaving = true;

    const normalized = {
      clipboard_monitor_enabled: settingsDraft.clipboard_monitor_enabled,
      show_main_window_on_start: settingsDraft.show_main_window_on_start,
      auto_start_enabled: settingsDraft.auto_start_enabled,
      max_items: numberSettingValue(settingsDraft.max_items, defaultSettings.max_items),
      capture_delay_ms: numberSettingValue(
        settingsDraft.capture_delay_ms,
        defaultSettings.capture_delay_ms
      ),
      screenshot_hotkey: settingsDraft.screenshot_hotkey || defaultSettings.screenshot_hotkey,
      main_window_hotkey:
        settingsDraft.main_window_hotkey || defaultSettings.main_window_hotkey,
      time_zone: settingsDraft.time_zone || defaultSettings.time_zone,
      language: settingsDraft.language || defaultSettings.language,
      auto_cleanup_enabled: settingsDraft.auto_cleanup_enabled,
      cleanup_max_items: numberSettingValue(
        settingsDraft.cleanup_max_items,
        defaultSettings.cleanup_max_items
      ),
      cleanup_keep_days: numberSettingValue(
        settingsDraft.cleanup_keep_days,
        defaultSettings.cleanup_keep_days
      ),
      dev_server_port: numberSettingValue(
        settingsDraft.dev_server_port,
        defaultSettings.dev_server_port
      ),
    };

    try {
      const timeZoneChanged = normalized.time_zone !== appSettings.time_zone;
      const devServerPortChanged = normalized.dev_server_port
        !== (appSettings.dev_server_port || defaultSettings.dev_server_port);
      const savedSettings = await settingsApi.saveSettings(normalized);
      let autoCleanupPlan = null;
      let autoCleanupError = null;

      if (savedSettings.auto_cleanup_enabled) {
        try {
          autoCleanupPlan = await settingsApi.runCustomCleanup(
            savedSettings.cleanup_max_items,
            savedSettings.cleanup_keep_days
          );
        } catch (cleanupError) {
          console.error('自动清理失败:', cleanupError);
          autoCleanupError = cleanupError;
        }
      }

      appSettings = savedSettings;
      settingsDraft = { ...appSettings };
      cleanupPlan = autoCleanupPlan;
      if (devServerPortChanged) {
        pendingRestartPort = savedSettings.dev_server_port;
        activeSettingsView = 'advanced';
      } else {
        onClose();
      }
      if (timeZoneChanged) {
        selectedDay = '';
      }
      await onRefreshRecords();

      if (autoCleanupError) {
        showActionError('设置已保存，自动清理失败: ' + autoCleanupError);
      } else if (autoCleanupPlan) {
        showActionNotice(`设置已保存，已清理 ${autoCleanupPlan.item_count} 条记录`);
      } else if (devServerPortChanged) {
        showActionNotice('端口已保存，重启后生效');
      } else {
        showActionNotice('设置已保存');
      }
    } catch (e) {
      console.error('保存设置失败:', e);
      showActionError('保存设置失败: ' + e);
    } finally {
      settingsSaving = false;
    }
  }

  async function previewCleanup() {
    cleanupLoading = true;

    try {
      cleanupPlan = await settingsApi.previewCustomCleanup(
        numberSettingValue(settingsDraft.cleanup_max_items, defaultSettings.cleanup_max_items),
        numberSettingValue(settingsDraft.cleanup_keep_days, defaultSettings.cleanup_keep_days)
      );
    } catch (e) {
      console.error('预览清理失败:', e);
      showActionError('预览清理失败: ' + e);
    } finally {
      cleanupLoading = false;
    }
  }

  async function runCleanupNow() {
    cleanupLoading = true;

    try {
      cleanupPlan = await settingsApi.runCustomCleanup(
        numberSettingValue(settingsDraft.cleanup_max_items, defaultSettings.cleanup_max_items),
        numberSettingValue(settingsDraft.cleanup_keep_days, defaultSettings.cleanup_keep_days)
      );
      await onRefreshRecords();
      showActionNotice(`已清理 ${cleanupPlan.item_count} 条记录`);
    } catch (e) {
      console.error('执行清理失败:', e);
      showActionError('执行清理失败: ' + e);
    } finally {
      cleanupLoading = false;
    }
  }

  function requestClearAllHistory() {
    cleanupPlan = null;
    clearHistoryConfirmOpen = true;
  }

  function cancelClearAllHistory() {
    if (clearHistoryLoading) return;
    clearHistoryConfirmOpen = false;
  }

  async function confirmClearAllHistory() {
    if (clearHistoryLoading) return;

    clearHistoryLoading = true;

    try {
      const plan = await settingsApi.clearAllHistory();
      clearHistoryConfirmOpen = false;
      selectedDay = '';
      await onHistoryCleared(plan);
    } catch (e) {
      console.error('清空历史失败:', e);
      showActionError('清空历史失败: ' + e);
    } finally {
      clearHistoryLoading = false;
    }
  }

  function startRecordingHotkey(field) {
    isRecordingHotkey = true;
    recordingHotkeyField = field;
    hotkeyRecordingMessage = '';

    if (recordingHotkeyTimeout) {
      clearTimeout(recordingHotkeyTimeout);
    }

    recordingHotkeyTimeout = setTimeout(() => {
      stopRecordingHotkey();
    }, 5000);
  }

  function stopRecordingHotkey() {
    isRecordingHotkey = false;
    recordingHotkeyField = null;
    hotkeyRecordingMessage = '';
    if (recordingHotkeyTimeout) {
      clearTimeout(recordingHotkeyTimeout);
      recordingHotkeyTimeout = null;
    }
  }

  function handleHotkeyKeyDown(event) {
    if (!isRecordingHotkey) return;

    event.preventDefault();

    const result = hotkeyFromKeyboardEvent(event);
    if (result.ignored) return;

    if (result.message) {
      hotkeyRecordingMessage = result.message;
      return;
    }

    updateSettingsDraft(recordingHotkeyField || 'screenshot_hotkey', result.hotkey);
    hotkeyRecordingMessage = '';
    stopRecordingHotkey();
  }

  async function openExternalLink(event, url) {
    event.preventDefault();
    try {
      await toolApi.openExternalUrl(url);
    } catch (e) {
      console.error('打开链接失败:', e);
      showActionError('打开链接失败: ' + e);
    }
  }
</script>

<SettingsPanel
  bind:activeSettingsView
  {appDataDir}
  {appDataDirError}
  {appVersion}
  {cleanupLoading}
  {cleanupPlan}
  {clearHistoryLoading}
  {githubIssuesUrl}
  {githubProfileUrl}
  {hotkeyRecordingMessage}
  {isRecordingHotkey}
  {languageOptions}
  {pendingRestartPort}
  {portCheckLoading}
  {portCheckResult}
  {recordingHotkeyField}
  {restartingApp}
  {settingsDraft}
  {settingsPortChanged}
  {settingsSaving}
  {settingsViews}
  {timeZoneOptions}
  onApplySuggestedPort={applySuggestedPort}
  onCheckDevServerPort={checkDevServerPort}
  onClose={onClose}
  onHotkeyBlur={stopRecordingHotkey}
  onHotkeyFocus={startRecordingHotkey}
  onHotkeyKeyDown={handleHotkeyKeyDown}
  onOpenExternalLink={openExternalLink}
  onPreviewCleanup={previewCleanup}
  onRequestClearAllHistory={requestClearAllHistory}
  onRestartApplication={restartApplication}
  onRunCleanupNow={runCleanupNow}
  onSaveSettings={saveSettings}
  onUpdateSettingsDraft={updateSettingsDraft}
/>

{#if clearHistoryConfirmOpen}
  <ClearHistoryConfirmDialog
    {clearHistoryLoading}
    onCancel={cancelClearAllHistory}
    onConfirm={confirmClearAllHistory}
  />
{/if}
