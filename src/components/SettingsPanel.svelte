<script>
  import { CalendarDays, Clipboard, GitPullRequest, Settings, Trash2, X } from '@lucide/svelte';

  export let activeSettingsView = 'basic';
  export let appDataDir = '';
  export let appDataDirError = '';
  export let appVersion = '';
  export let cleanupLoading = false;
  export let cleanupPlan = null;
  export let clearHistoryLoading = false;
  export let githubIssuesUrl = '';
  export let githubProfileUrl = '';
  export let hotkeyRecordingMessage = '';
  export let isRecordingHotkey = false;
  export let languageOptions = [];
  export let pendingRestartPort = null;
  export let portCheckLoading = false;
  export let portCheckResult = null;
  export let recordingHotkeyField = null;
  export let restartingApp = false;
  export let settingsDraft = {};
  export let settingsPortChanged = false;
  export let settingsSaving = false;
  export let settingsViews = [];
  export let timeZoneOptions = [];

  export let onApplySuggestedPort = () => {};
  export let onCheckDevServerPort = () => {};
  export let onClose = () => {};
  export let onHotkeyBlur = () => {};
  export let onHotkeyFocus = () => {};
  export let onHotkeyKeyDown = () => {};
  export let onOpenExternalLink = () => {};
  export let onPreviewCleanup = () => {};
  export let onRequestClearAllHistory = () => {};
  export let onRestartApplication = () => {};
  export let onRunCleanupNow = () => {};
  export let onSaveSettings = () => {};
  export let onUpdateSettingsDraft = () => {};

  function optionLabel(options, value) {
    return options.find((option) => option.value === value)?.label || value;
  }
</script>

<div class="settings-backdrop" on:click={onClose} aria-hidden="true"></div>
<div
  class="settings-panel"
  role="dialog"
  aria-modal="true"
  aria-labelledby="settings-title"
>
  <header class="settings-header">
    <div>
      <p class="eyebrow">Preferences</p>
      <h2 id="settings-title">设置</h2>
    </div>
    <button type="button" on:click={onClose} aria-label="关闭设置">
      <X size={16} aria-hidden="true" />
    </button>
  </header>

  <div class="settings-workspace">
    <div class="settings-nav" aria-label="设置分类" role="tablist">
      {#each settingsViews as view}
        <button
          type="button"
          role="tab"
          id={`settings-tab-${view.id}`}
          class:active={activeSettingsView === view.id}
          aria-selected={activeSettingsView === view.id}
          aria-controls={`settings-view-${view.id}`}
          on:click={() => (activeSettingsView = view.id)}
        >
          {#if view.id === 'basic'}
            <Settings class="settings-tab-icon" size={15} aria-hidden="true" />
          {:else if view.id === 'locale'}
            <CalendarDays class="settings-tab-icon" size={15} aria-hidden="true" />
          {:else if view.id === 'advanced'}
            <Settings class="settings-tab-icon" size={15} aria-hidden="true" />
          {:else}
            <Clipboard class="settings-tab-icon" size={15} aria-hidden="true" />
          {/if}
          <span>{view.label}</span>
        </button>
      {/each}
    </div>

    <div class="settings-content">
      {#if activeSettingsView === 'basic'}
        <div
          class="settings-section settings-view"
          id="settings-view-basic"
          role="tabpanel"
          aria-labelledby="settings-tab-basic"
        >
          <div class="settings-section-title">
            <h3>常规设置</h3>
            <p>启动 / 监听 / 截图</p>
          </div>

          <label class="switch-row">
            <input
              type="checkbox"
              checked={settingsDraft.clipboard_monitor_enabled}
              on:change={(event) =>
                onUpdateSettingsDraft('clipboard_monitor_enabled', event.currentTarget.checked)}
            />
            <span>监听剪贴板</span>
          </label>

          <label class="switch-row">
            <input
              type="checkbox"
              checked={settingsDraft.show_main_window_on_start}
              on:change={(event) =>
                onUpdateSettingsDraft('show_main_window_on_start', event.currentTarget.checked)}
            />
            <span>启动时显示主窗口</span>
          </label>

          <label class="switch-row">
            <input
              type="checkbox"
              checked={settingsDraft.auto_start_enabled}
              on:change={(event) =>
                onUpdateSettingsDraft('auto_start_enabled', event.currentTarget.checked)}
            />
            <span>开机自启动</span>
          </label>

          <label class="field-row">
            <span>保留记录数</span>
            <input
              type="number"
              min="10"
              max="500"
              value={settingsDraft.max_items}
              on:input={(event) =>
                onUpdateSettingsDraft('max_items', Number(event.currentTarget.value))}
            />
          </label>

          <label class="field-row">
            <span>截图延迟</span>
            <input
              type="number"
              min="0"
              max="3000"
              step="50"
              value={settingsDraft.capture_delay_ms}
              on:input={(event) =>
                onUpdateSettingsDraft('capture_delay_ms', Number(event.currentTarget.value))}
            />
          </label>

          <div class="settings-section-title inline-section-title">
            <h3>快捷键</h3>
            <p>截图和主窗口入口</p>
          </div>

          <label class="field-row">
            <span>截图</span>
            <input
              type="text"
              readonly
              placeholder="点击后按下组合键"
              value={settingsDraft.screenshot_hotkey}
              on:focus={() => onHotkeyFocus('screenshot_hotkey')}
              on:blur={onHotkeyBlur}
              on:keydown={onHotkeyKeyDown}
              class:recording={isRecordingHotkey && recordingHotkeyField === 'screenshot_hotkey'}
            />
          </label>

          <label class="field-row">
            <span>主窗口</span>
            <input
              type="text"
              readonly
              placeholder="点击后按下组合键"
              value={settingsDraft.main_window_hotkey}
              on:focus={() => onHotkeyFocus('main_window_hotkey')}
              on:blur={onHotkeyBlur}
              on:keydown={onHotkeyKeyDown}
              class:recording={isRecordingHotkey && recordingHotkeyField === 'main_window_hotkey'}
            />
          </label>
          <p class="hotkey-hint" aria-live="polite">
            {#if isRecordingHotkey}
              {hotkeyRecordingMessage || '正在录制，请按下组合键（如 Ctrl+Shift+A）'}
            {:else}
              点击输入框后按下组合键自动录制，例如 Ctrl+Shift+A
            {/if}
          </p>
        </div>
      {:else if activeSettingsView === 'locale'}
        <div
          class="settings-section settings-view"
          id="settings-view-locale"
          role="tabpanel"
          aria-labelledby="settings-tab-locale"
        >
          <div class="settings-section-title">
            <h3>界面与日期</h3>
            <p>语言 / 自然日</p>
          </div>

          <label class="field-row">
            <span>日期划分时区</span>
            <select
              value={settingsDraft.time_zone}
              on:change={(event) => onUpdateSettingsDraft('time_zone', event.currentTarget.value)}
            >
              {#each timeZoneOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </label>

          <label class="field-row">
            <span>应用语言</span>
            <select
              value={settingsDraft.language}
              on:change={(event) => onUpdateSettingsDraft('language', event.currentTarget.value)}
            >
              {#each languageOptions as option}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </label>
        </div>
      {:else if activeSettingsView === 'advanced'}
        <div
          class="settings-section settings-view advanced-settings"
          id="settings-view-advanced"
          role="tabpanel"
          aria-labelledby="settings-tab-advanced"
        >
          <section class="settings-card advanced-card" aria-label="记录清理设置">
            <header class="settings-card-header">
              <div>
                <h3 id="cleanup-settings-title">记录清理</h3>
                <p>只清理普通记录，置顶和收藏会保留。</p>
              </div>
              <label class="switch-row compact-switch">
                <input
                  type="checkbox"
                  aria-label="保存设置后自动清理"
                  checked={settingsDraft.auto_cleanup_enabled}
                  on:change={(event) =>
                    onUpdateSettingsDraft('auto_cleanup_enabled', event.currentTarget.checked)}
                />
                <span>自动清理</span>
              </label>
            </header>

            <div class="settings-field-grid">
              <label class="field-row compact-field">
                <span>最多保留</span>
                <input
                  type="number"
                  aria-label="普通记录最多保留"
                  min="10"
                  max="5000"
                  value={settingsDraft.cleanup_max_items}
                  on:input={(event) =>
                    onUpdateSettingsDraft('cleanup_max_items', Number(event.currentTarget.value))}
                />
              </label>

              <label class="field-row compact-field">
                <span>保留天数</span>
                <input
                  type="number"
                  aria-label="普通记录保留天数"
                  min="1"
                  max="3650"
                  value={settingsDraft.cleanup_keep_days}
                  on:input={(event) =>
                    onUpdateSettingsDraft('cleanup_keep_days', Number(event.currentTarget.value))}
                />
              </label>
            </div>

            <p class="cleanup-hint">图片文件会随被清理的图片记录同步删除。</p>

            {#if cleanupPlan}
              <p class="cleanup-plan" role="status">
                将清理 {cleanupPlan.item_count} 条记录（文本 {cleanupPlan.text_count}，图片 {cleanupPlan.image_count}）
              </p>
            {/if}

            <div class="cleanup-actions">
              <button type="button" class="ghost-button" on:click={onPreviewCleanup} disabled={cleanupLoading}>
                {cleanupLoading ? '计算中' : '预览清理'}
              </button>
              <button type="button" class="ghost-button" on:click={onRunCleanupNow} disabled={cleanupLoading}>
                {cleanupLoading ? '清理中' : '立即清理'}
              </button>
            </div>
          </section>

          <section class="settings-card advanced-card danger-card" aria-label="危险操作">
            <header class="settings-card-header">
              <div>
                <h3>危险操作</h3>
                <p>清空全部记录、收藏、置顶、标注和图片文件。</p>
              </div>
            </header>

            <div class="danger-actions">
              <p class="danger-copy">这个操作无法撤销，当前窗口会立即刷新为空历史。</p>
              <button
                type="button"
                class="danger-button clear-history-button"
                on:click={onRequestClearAllHistory}
                disabled={clearHistoryLoading}
              >
                <Trash2 size={15} aria-hidden="true" />
                {clearHistoryLoading ? '清空中' : '清空全部历史'}
              </button>
            </div>
          </section>

          <section class="settings-card advanced-card" aria-label="开发端口设置">
            <header class="settings-card-header">
              <div>
                <h3 id="port-settings-title">开发端口</h3>
                <p>检查占用状态，保存后重启生效。</p>
              </div>
            </header>

            <div class="field-row port-field compact-field">
              <label for="dev-server-port">端口</label>
              <div class="port-input-group">
                <input
                  id="dev-server-port"
                  type="number"
                  aria-label="开发端口"
                  min="1"
                  max="65535"
                  value={settingsDraft.dev_server_port}
                  on:input={(event) =>
                    onUpdateSettingsDraft('dev_server_port', Number(event.currentTarget.value))}
                />
                <button
                  type="button"
                  class="ghost-button compact-button"
                  on:click={onCheckDevServerPort}
                  disabled={portCheckLoading}
                >
                  {portCheckLoading ? '检查中' : '检查端口'}
                </button>
              </div>
            </div>

            <div
              class:available={portCheckResult?.available}
              class:occupied={portCheckResult && !portCheckResult.available}
              class="port-check-result"
              aria-live="polite"
            >
              {#if portCheckResult}
                <p>{portCheckResult.message}</p>
                {#if !portCheckResult.available && portCheckResult.suggested_port}
                  <button
                    type="button"
                    class="ghost-button compact-button"
                    on:click={() => onApplySuggestedPort(portCheckResult.suggested_port)}
                  >
                    使用 {portCheckResult.suggested_port}
                  </button>
                {/if}
              {:else}
                <p>生产版不会占用本地开发端口。</p>
              {/if}
            </div>

            {#if settingsPortChanged}
              <p class="port-hint">端口变化需要保存并重启应用后生效。</p>
            {/if}

            {#if pendingRestartPort}
              <div class="restart-card" role="status">
                <div>
                  <strong>端口 {pendingRestartPort} 已保存</strong>
                  <span>重启后应用会切到新的开发端口。</span>
                </div>
                <button
                  type="button"
                  class="primary-button"
                  on:click={onRestartApplication}
                  disabled={restartingApp}
                >
                  {restartingApp ? '重启中' : '重启应用'}
                </button>
              </div>
            {/if}
          </section>
        </div>
      {:else}
        <div
          class="settings-section settings-view about-section"
          id="settings-view-about"
          role="tabpanel"
          aria-labelledby="settings-tab-about"
        >
          <div class="about-profile">
            <img class="about-avatar" src="/github-avatar-display.jpg" alt="s1oopX GitHub 头像" />
            <div class="about-profile-copy">
              <span class="about-eyebrow">GitHub · s1oopX</span>
              <h3>s1oopX</h3>
              <p>
                ClipMaster 的作者与维护者。这个工具保持轻巧，只处理复制、截图、标注和贴图这些基础事情。
              </p>
            </div>
          </div>

          <div class="about-block">
            <h4>项目简介</h4>
            <p>
              ClipMaster 是一个轻巧的本地剪贴板工具，用来保存复制记录、截图、基础标注和贴图。数据默认保存在本机，记录按日期整理。
            </p>
          </div>

          <dl class="about-list">
            <div>
              <dt>版本</dt>
              <dd>{appVersion}</dd>
            </div>
            <div>
              <dt>数据</dt>
              <dd>本地保存</dd>
            </div>
            <div>
              <dt>数据目录</dt>
              <dd class="path-value">
                {appDataDir || (appDataDirError ? '读取失败，请查看排障文档' : '加载中')}
              </dd>
            </div>
            <div>
              <dt>日期规则</dt>
              <dd>{optionLabel(timeZoneOptions, settingsDraft.time_zone)}</dd>
            </div>
          </dl>

          <div class="about-block about-contact">
            <h4>联系方式</h4>
            <div class="about-links">
              <a
                class="about-link"
                href={githubProfileUrl}
                target="_blank"
                rel="noreferrer"
                on:click={(event) => onOpenExternalLink(event, githubProfileUrl)}
              >
                <GitPullRequest size={14} aria-hidden="true" />
                <span>
                  <strong>GitHub 主页</strong>
                  <small>s1oopX</small>
                </span>
              </a>
              <a
                class="about-link"
                href={githubIssuesUrl}
                target="_blank"
                rel="noreferrer"
                on:click={(event) => onOpenExternalLink(event, githubIssuesUrl)}
              >
                <GitPullRequest size={14} aria-hidden="true" />
                <span>
                  <strong>提交问题或建议</strong>
                  <small>s1oopX/clipmaster-tauri</small>
                </span>
              </a>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <footer class="settings-footer">
    <button type="button" class="ghost-button" on:click={onClose}>
      取消
    </button>
    <button type="button" class="primary-button" on:click={onSaveSettings} disabled={settingsSaving}>
      {settingsSaving ? '保存中' : '保存设置'}
    </button>
  </footer>
</div>
