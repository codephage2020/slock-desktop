import { type CSSProperties, startTransition, useEffect, useState } from 'react'
import './App.css'
import './Settings.css'
import {
  type BootstrapPayload,
  type ServiceSnapshot,
  type ThemeDefinition,
  loadBootstrap,
  openExternalUrl,
  openWorkspace,
  saveCustomTheme,
  saveServiceSettings,
  saveUpdateSettings,
  startService,
  stopService,
  updateTheme,
  updateLanguage,
  updateThemeMode,
} from './lib/desktop'

interface ReleaseAsset {
  name: string
  browserDownloadUrl: string
}

interface ReleaseInfo {
  tagName: string
  name: string
  htmlUrl: string
  publishedAt: string
  body: string
  prerelease: boolean
  assets: ReleaseAsset[]
  updateAvailable: boolean
}

interface ReleaseState {
  loading: boolean
  error: string | null
  latest: ReleaseInfo | null
}

const INITIAL_RELEASE_STATE: ReleaseState = {
  loading: false,
  error: null,
  latest: null,
}

const THEME_MODES = [
  { id: 'light', icon: '☼', labelKey: 'modeLight' },
  { id: 'dark', icon: '◐', labelKey: 'modeDark' },
  { id: 'system', icon: '◌', labelKey: 'modeSystem' },
] as const

const LANGUAGE_OPTIONS = [
  { id: 'en-US', labelKey: 'languageEnglish', shortLabelKey: 'languageEnglishShort' },
  { id: 'zh-CN', labelKey: 'languageChinese', shortLabelKey: 'languageChineseShort' },
  { id: 'system', labelKey: 'languageSystem', shortLabelKey: 'languageSystemShort' },
] as const

const COPY = {
  'en-US': {
    header: 'Desktop Console',
    loadingTitle: 'Desktop Console is booting.',
    loadingDescription: 'Preparing the local shell and reading your desktop preferences.',
    lede:
      'Open the original Slock workspace, choose a theme color, and keep the desktop shell aligned with light, dark, or system mode.',
    workspaceActive: 'Workspace active',
    workspaceParked: 'Workspace parked',
    settings: 'Settings',
    settingsSections: 'Desktop settings sections',
    appearance: 'Appearance',
    service: 'Service',
    updates: 'Updates',
    desktopSettings: 'Desktop Settings',
    appearanceDescription:
      'Theme settings apply to the desktop shell, nested settings, and launched workspace.',
    mode: 'Mode',
    modeDescription: 'Choose light, dark, or follow the operating system.',
    themeColor: 'Theme color',
    themeDescription: 'Pick a color system for the shell, settings, and workspace.',
    customTheme: 'Custom theme',
    customThemeDescription: 'Define a personal accent and save it as the Custom theme.',
    customThemeName: 'Name',
    customThemeNamePlaceholder: 'Custom',
    customThemeAccent: 'Accent',
    customThemeAccentAria: 'Custom theme accent color',
    language: 'Language',
    languageDescription: 'Choose Chinese, English, or follow the operating system.',
    applyScope: 'Apply scope',
    applyDescription:
      'Current theme covers startup page, settings, workspace overlay, and remote page injection.',
    savedLocally: 'Saved locally',
    saveCustomTheme: 'Save Custom Theme',
    saving: 'Saving…',
    modeSuffix: 'mode',
    modeLight: 'Light',
    modeDark: 'Dark',
    modeSystem: 'System',
    languageEnglish: 'English',
    languageChinese: 'Chinese',
    languageSystem: 'System',
    languageEnglishShort: 'EN',
    languageChineseShort: '中',
    languageSystemShort: 'System',
    focusSlock: 'Focus Slock',
    openSlock: 'Open Slock',
    launching: 'Launching…',
    running: 'Running',
    configuredIdle: 'Configured / idle',
    notConfigured: 'Not configured',
    desktopStateError: 'Desktop state error',
    previewLabel: 'preview',
    previewUserText: 'Messages, tasks, and threads keep the same visual rhythm.',
    previewAssistantText: 'The workspace stays clear for long daily sessions.',
    previewing: 'Interface preview',
    previewSendButton: 'Preview send button',
    localServiceEyebrow: 'Local Service',
    serviceStartup: 'Service startup',
    serviceRunning: 'running',
    serviceIdle: 'idle',
    serviceCopy: 'Optional local service command for workspace launch.',
    commandPath: 'Command path',
    commandPathPlaceholder: '/absolute/path/to/service',
    workingDirectory: 'Working directory',
    workingDirectoryPlaceholder: '/absolute/path/to/project',
    arguments: 'Arguments',
    argumentsPlaceholder: 'One argument per line\n--port\n3141',
    autoStartService: 'Auto-start the service when launching the workspace',
    serviceSaved: 'Service command saved locally.',
    cloudWorkspaceOnly: 'Leave empty for cloud workspace only.',
    saveServiceSettings: 'Save Service Settings',
    savingServiceSettings: 'Saving…',
    startService: 'Start Service',
    startingService: 'Starting…',
    stopService: 'Stop Service',
    stoppingService: 'Stopping…',
    updateCenterEyebrow: 'Update Center',
    releaseCheck: 'Release check',
    updateAvailable: 'update available',
    current: 'current',
    notChecked: 'not checked',
    releaseCopy: 'Check the configured GitHub release channel.',
    repository: 'Repository',
    releasesPage: 'Releases page',
    installed: 'Installed',
    latestCheckApi: 'Latest check API',
    prerelease: 'prerelease',
    published: 'Published',
    noReleaseNotes: 'No release notes were provided for this release.',
    noReleaseCheck: 'No release check yet.',
    saveUpdateSettings: 'Save Update Settings',
    savingUpdateSettings: 'Saving…',
    checkGitHubRelease: 'Check GitHub Release',
    checkingRelease: 'Checking…',
    openReleases: 'Open Releases',
    unknownDate: 'unknown date',
  },
  'zh-CN': {
    header: '桌面控制台',
    loadingTitle: '桌面控制台正在启动。',
    loadingDescription: '正在准备本地外壳并读取你的桌面偏好。',
    lede: '打开原始 Slock 工作区，选择主题色，并让桌面壳在亮色、暗黑或跟随系统模式下保持一致。',
    workspaceActive: '工作区已打开',
    workspaceParked: '工作区待启动',
    settings: '设置',
    settingsSections: '桌面设置分区',
    appearance: '外观',
    service: '服务',
    updates: '更新',
    desktopSettings: '桌面设置',
    appearanceDescription: '主题设置会应用到桌面壳、内嵌设置页和启动后的工作区。',
    mode: '模式',
    modeDescription: '选择亮色、暗黑，或跟随操作系统。',
    themeColor: '主题色彩',
    themeDescription: '为桌面壳、设置页和工作区选择统一色彩。',
    customTheme: '自定义主题',
    customThemeDescription: '定义个人强调色，并保存为自定义主题。',
    customThemeName: '名称',
    customThemeNamePlaceholder: '自定义',
    customThemeAccent: '强调色',
    customThemeAccentAria: '自定义主题强调色',
    language: '语言',
    languageDescription: '选择中文、英文，或跟随操作系统。',
    applyScope: '应用范围',
    applyDescription: '当前主题覆盖起始页、设置页、工作区浮层和远端页面注入。',
    savedLocally: '已保存本地',
    saveCustomTheme: '保存自定义主题',
    saving: '保存中…',
    modeSuffix: '模式',
    modeLight: '亮色',
    modeDark: '暗黑',
    modeSystem: '系统',
    languageEnglish: '英文',
    languageChinese: '中文',
    languageSystem: '系统',
    languageEnglishShort: 'EN',
    languageChineseShort: '中',
    languageSystemShort: '跟随系统',
    focusSlock: '聚焦 Slock',
    openSlock: '打开 Slock',
    launching: '启动中…',
    running: '运行中',
    configuredIdle: '已配置 / 空闲',
    notConfigured: '未配置',
    desktopStateError: '桌面状态错误',
    previewLabel: '预览',
    previewUserText: '消息、任务和线程保持一致的阅读节奏。',
    previewAssistantText: '工作区保持清晰，适合长时间使用。',
    previewing: '界面预览',
    previewSendButton: '预览发送按钮',
    localServiceEyebrow: '本地服务',
    serviceStartup: '服务启动',
    serviceRunning: '运行中',
    serviceIdle: '空闲',
    serviceCopy: '可选的本地服务命令，用于启动工作区时联动。',
    commandPath: '命令路径',
    commandPathPlaceholder: '/绝对路径/服务命令',
    workingDirectory: '工作目录',
    workingDirectoryPlaceholder: '/绝对路径/项目',
    arguments: '参数',
    argumentsPlaceholder: '每行一个参数\n--port\n3141',
    autoStartService: '启动工作区时自动启动服务',
    serviceSaved: '服务命令已保存到本地。',
    cloudWorkspaceOnly: '留空时只打开云端工作区。',
    saveServiceSettings: '保存服务设置',
    savingServiceSettings: '保存中…',
    startService: '启动服务',
    startingService: '启动中…',
    stopService: '停止服务',
    stoppingService: '停止中…',
    updateCenterEyebrow: '更新中心',
    releaseCheck: '版本检查',
    updateAvailable: '有可用更新',
    current: '已是最新',
    notChecked: '未检查',
    releaseCopy: '检查已配置的 GitHub Release 通道。',
    repository: '仓库',
    releasesPage: '发布页',
    installed: '已安装',
    latestCheckApi: '最新版本 API',
    prerelease: '预发布',
    published: '发布于',
    noReleaseNotes: '此版本没有提供发布说明。',
    noReleaseCheck: '尚未检查版本。',
    saveUpdateSettings: '保存更新设置',
    savingUpdateSettings: '保存中…',
    checkGitHubRelease: '检查 GitHub Release',
    checkingRelease: '检查中…',
    openReleases: '打开发布页',
    unknownDate: '未知日期',
  },
} as const

const ZH_THEME_COPY: Record<string, { name: string; summary: string }> = {
  default: {
    name: '默认',
    summary: '适合日常桌面工作的克制绿色强调色。',
  },
  light: {
    name: '雾蓝',
    summary: '适合安静操作视图的柔和蓝色强调色。',
  },
  dark: {
    name: '靛蓝',
    summary: '适合结构化专注的低饱和靛蓝强调色。',
  },
  graphite: {
    name: '石墨',
    summary: '适合长时间会话的低饱和灰蓝强调色。',
  },
  crimson: {
    name: '玫瑰',
    summary: '适合编辑型工作区的温暖玫瑰强调色。',
  },
  custom: {
    name: '自定义',
    summary: '用户定义的个人强调色主题。',
  },
}

function App() {
  const [snapshot, setSnapshot] = useState<BootstrapPayload | null>(null)
  const [busyAction, setBusyAction] = useState<string | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [releaseState, setReleaseState] = useState<ReleaseState>(INITIAL_RELEASE_STATE)

  useEffect(() => {
    let cancelled = false

    void loadBootstrap()
      .then((next) => {
        if (!cancelled) {
          startTransition(() => setSnapshot(next))
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setErrorMessage(getErrorMessage(error))
        }
      })

    return () => {
      cancelled = true
    }
  }, [])

  async function handleThemeChange(themeId: string) {
    try {
      setBusyAction(themeId)
      setErrorMessage(null)
      const next = await updateTheme(themeId)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleThemeModeChange(
    themeMode: BootstrapPayload['appearanceMode'],
  ) {
    try {
      setBusyAction(`mode:${themeMode}`)
      setErrorMessage(null)
      const next = await updateThemeMode(themeMode)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleLanguageChange(
    language: BootstrapPayload['language'],
  ) {
    try {
      setBusyAction(`language:${language}`)
      setErrorMessage(null)
      const next = await updateLanguage(language)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleCustomThemeSave() {
    if (!snapshot) {
      return
    }

    try {
      setBusyAction('custom-theme')
      setErrorMessage(null)
      const next = await saveCustomTheme(snapshot.customTheme)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleWorkspaceOpen() {
    try {
      setBusyAction('workspace')
      setErrorMessage(null)
      const next = await openWorkspace()
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleServiceSave() {
    if (!snapshot) {
      return
    }

    try {
      setBusyAction('save-service')
      setErrorMessage(null)
      const next = await saveServiceSettings(snapshot.service)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleServiceStart() {
    try {
      setBusyAction('start-service')
      setErrorMessage(null)
      const next = await startService()
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleServiceStop() {
    try {
      setBusyAction('stop-service')
      setErrorMessage(null)
      const next = await stopService()
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleUpdateSettingsSave() {
    if (!snapshot) {
      return
    }

    try {
      setBusyAction('save-updates')
      setErrorMessage(null)
      const next = await saveUpdateSettings(snapshot.updates)
      startTransition(() => setSnapshot(next))
      setReleaseState(INITIAL_RELEASE_STATE)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleReleaseCheck() {
    if (!snapshot) {
      return
    }

    try {
      setReleaseState((current) => ({
        ...current,
        loading: true,
        error: null,
      }))

      const response = await fetch(snapshot.updates.latestReleaseApiUrl, {
        headers: {
          Accept: 'application/vnd.github+json',
        },
      })

      if (!response.ok) {
        throw new Error(`GitHub release check failed with ${response.status}`)
      }

      const payload = await response.json()
      const latest = mapReleasePayload(payload, snapshot.updates.currentVersion)
      setReleaseState({
        loading: false,
        error: null,
        latest,
      })
    } catch (error) {
      setReleaseState({
        loading: false,
        error: getErrorMessage(error),
        latest: null,
      })
    }
  }

  async function handleOpenExternal(url: string) {
    try {
      setBusyAction(`open:${url}`)
      setErrorMessage(null)
      await openExternalUrl(url)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  function patchService(patch: Partial<ServiceSnapshot>) {
    setSnapshot((current) =>
      current
        ? {
            ...current,
            service: {
              ...current.service,
              ...patch,
            },
          }
        : current,
    )
  }

  function patchUpdates(
    patch: Partial<BootstrapPayload['updates']>,
  ) {
    setSnapshot((current) =>
      current
        ? {
            ...current,
            updates: {
              ...current.updates,
              ...patch,
            },
          }
        : current,
    )
  }

  function patchCustomTheme(
    patch: Partial<BootstrapPayload['customTheme']>,
  ) {
    setSnapshot((current) =>
      current
        ? {
            ...current,
            customTheme: {
              ...current.customTheme,
              ...patch,
            },
          }
        : current,
    )
  }

  if (!snapshot) {
    const bootCopy = getCopy('system')

    return (
      <main className="loading-shell">
        <p className="eyebrow">SLOCK DESKTOP</p>
        <h1>{bootCopy.loadingTitle}</h1>
        <p>{bootCopy.loadingDescription}</p>
      </main>
    )
  }

  const activeTheme =
    snapshot.themes.find((theme) => theme.id === snapshot.colorScheme) ??
    snapshot.themes[0]
  const copy = getCopy(snapshot.language, snapshot.resolvedLanguage)
  const activeThemeDisplay = getThemeDisplay(activeTheme, snapshot.language, snapshot.resolvedLanguage)

  const shellStyle = buildShellStyle(activeTheme)
  const stackButtonLabel = snapshot.workspaceOpen ? copy.focusSlock : copy.openSlock

  return (
    <main className="studio-shell" data-mode={activeTheme.mode} style={shellStyle}>
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      {errorMessage ? (
        <section className="error-banner" role="alert">
          <strong>{copy.desktopStateError}</strong>
          <p>{errorMessage}</p>
        </section>
      ) : null}

      <section className="launch-board" aria-label={copy.openSlock}>
        <section className="launch-center-card">
          <div className="status-row">
            <span className="status-dot" />
            <span>{snapshot.workspaceOpen ? copy.workspaceActive : copy.workspaceParked}</span>
          </div>

          <button
            className="launch-button"
            onClick={handleWorkspaceOpen}
            disabled={busyAction === 'workspace'}
          >
            {busyAction === 'workspace' ? copy.launching : stackButtonLabel}
          </button>

        </section>

        <section className="appearance-card launch-panel" aria-labelledby="appearance-settings-title">
          <div className="launch-panel-head">
            <div>
              <p className="eyebrow">{copy.desktopSettings}</p>
              <h2 id="appearance-settings-title">{copy.appearance}</h2>
            </div>
            <span className="settings-save-state">{copy.savedLocally}</span>
          </div>

          <div className="settings-quick-controls">
            <div className="icon-segment" role="radiogroup" aria-label={copy.mode}>
              {THEME_MODES.map((mode) => {
                const selected = mode.id === snapshot.appearanceMode
                return (
                  <button
                    key={mode.id}
                    className={`icon-option${selected ? ' selected' : ''}`}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    title={copy[mode.labelKey]}
                    onClick={() => handleThemeModeChange(mode.id)}
                    disabled={busyAction === `mode:${mode.id}`}
                  >
                    <span aria-hidden="true">{mode.icon}</span>
                    <span className="sr-only">{copy[mode.labelKey]}</span>
                  </button>
                )
              })}
            </div>
            <div className="icon-segment" role="radiogroup" aria-label={copy.language}>
              {LANGUAGE_OPTIONS.map((language) => {
                const selected = language.id === snapshot.language
                return (
                  <button
                    key={language.id}
                    className={`icon-option text-icon${selected ? ' selected' : ''}`}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    title={copy[language.labelKey]}
                    onClick={() => handleLanguageChange(language.id)}
                    disabled={busyAction === `language:${language.id}`}
                  >
                    <span aria-hidden="true">{copy[language.shortLabelKey]}</span>
                    <span className="sr-only">{copy[language.labelKey]}</span>
                  </button>
                )
              })}
            </div>
          </div>

          <div className="compact-setting-group">
            <div className="setting-copy compact-copy">
              <p className="setting-label">{copy.themeColor}</p>
              <p>{copy.themeDescription}</p>
            </div>

            <div className="theme-picker" role="radiogroup" aria-label={copy.themeColor}>
              {snapshot.themes.map((theme) => {
                const selected = theme.id === snapshot.colorScheme
                const themeDisplay = getThemeDisplay(theme, snapshot.language, snapshot.resolvedLanguage)
                return (
                  <button
                    key={theme.id}
                    className={`theme-option${selected ? ' selected' : ''}`}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() => handleThemeChange(theme.id)}
                    disabled={busyAction === theme.id}
                    style={buildThemeOptionStyle(theme)}
                  >
                    <span className="theme-option-preview" aria-hidden="true">
                      <span />
                      <span />
                      <span />
                    </span>
                    <span className="theme-option-copy">
                      <span className="theme-option-name">{themeDisplay.name}</span>
                      <span className="theme-option-summary">{themeDisplay.summary}</span>
                    </span>
                    <span className="theme-option-check" aria-hidden="true">
                      {selected ? '✓' : ''}
                    </span>
                  </button>
                )
              })}
            </div>
          </div>

          <div className="compact-setting-group">
            <div className="setting-copy compact-copy">
              <p className="setting-label">{copy.customTheme}</p>
              <p>{copy.customThemeDescription}</p>
            </div>

            <div className="custom-theme-controls">
              <label className="field compact-field">
                <span>{copy.customThemeName}</span>
                <input
                  value={snapshot.customTheme.name}
                  onChange={(event) =>
                    patchCustomTheme({ name: event.target.value })
                  }
                  placeholder={copy.customThemeNamePlaceholder}
                />
              </label>

              <label className="field compact-field color-field">
                <span>{copy.customThemeAccent}</span>
                <input
                  type="color"
                  value={snapshot.customTheme.accent}
                  onChange={(event) =>
                    patchCustomTheme({ accent: event.target.value })
                  }
                  aria-label={copy.customThemeAccentAria}
                />
              </label>

              <button
                className="theme-button"
                type="button"
                onClick={handleCustomThemeSave}
                disabled={busyAction === 'custom-theme'}
              >
                {busyAction === 'custom-theme' ? copy.saving : copy.saveCustomTheme}
              </button>
            </div>
          </div>
        </section>

        <aside className="appearance-preview" aria-label={`${activeThemeDisplay.name} ${copy.previewLabel}`}>
          <div className="preview-toolbar">
            <span />
            <span />
            <span />
          </div>
          <div className="preview-workspace">
            <div className="preview-sidebar">
              <span className="preview-pill wide" />
              <span className="preview-pill" />
              <span className="preview-pill short" />
            </div>
            <div className="preview-thread">
              <div className="preview-message user">
                <span />
                <p>{copy.previewUserText}</p>
              </div>
              <div className="preview-message assistant">
                <span />
                <p>{copy.previewAssistantText}</p>
              </div>
              <div className="preview-composer">
                <span>{copy.previewing}</span>
                <button type="button" aria-label={copy.previewSendButton}>↵</button>
              </div>
            </div>
          </div>
        </aside>

        <details className="control-card compact-control service-card">
          <summary className="control-card-head">
            <div>
              <p className="eyebrow">{copy.localServiceEyebrow}</p>
              <h2>{copy.serviceStartup}</h2>
            </div>
            <span className={`status-chip ${snapshot.service.running ? 'live' : ''}`}>
              {snapshot.service.running ? copy.serviceRunning : copy.serviceIdle}
            </span>
          </summary>

          <div className="control-body">
            <p className="control-copy">
              {copy.serviceCopy}
            </p>

            <label className="field">
              <span>{copy.commandPath}</span>
              <input
                value={snapshot.service.commandPath}
                onChange={(event) =>
                  patchService({ commandPath: event.target.value })
                }
                placeholder={copy.commandPathPlaceholder}
              />
            </label>

            <label className="field">
              <span>{copy.workingDirectory}</span>
              <input
                value={snapshot.service.workingDirectory}
                onChange={(event) =>
                  patchService({ workingDirectory: event.target.value })
                }
                placeholder={copy.workingDirectoryPlaceholder}
              />
            </label>

            <label className="field">
              <span>{copy.arguments}</span>
              <textarea
                value={snapshot.service.args.join('\n')}
                onChange={(event) =>
                  patchService({ args: splitArgs(event.target.value) })
                }
                placeholder={copy.argumentsPlaceholder}
              />
            </label>

            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={snapshot.service.autoStartWithWorkspace}
                onChange={(event) =>
                  patchService({ autoStartWithWorkspace: event.target.checked })
                }
              />
              <span>{copy.autoStartService}</span>
            </label>

            {snapshot.service.lastError ? (
              <p className="inline-note error">{snapshot.service.lastError}</p>
            ) : (
              <p className="inline-note">
                {snapshot.service.configured
                  ? copy.serviceSaved
                  : copy.cloudWorkspaceOnly}
              </p>
            )}

            <div className="button-row">
              <button
                className="theme-button"
                onClick={handleServiceSave}
                disabled={busyAction === 'save-service'}
              >
                {busyAction === 'save-service'
                  ? copy.savingServiceSettings
                  : copy.saveServiceSettings}
              </button>
              <button
                className="theme-button"
                onClick={handleServiceStart}
                disabled={busyAction === 'start-service'}
              >
                {busyAction === 'start-service' ? copy.startingService : copy.startService}
              </button>
              <button
                className="theme-button muted-button"
                onClick={handleServiceStop}
                disabled={busyAction === 'stop-service' || !snapshot.service.running}
              >
                {busyAction === 'stop-service' ? copy.stoppingService : copy.stopService}
              </button>
            </div>
          </div>
        </details>

        <details className="control-card compact-control update-card">
          <summary className="control-card-head">
            <div>
              <p className="eyebrow">{copy.updateCenterEyebrow}</p>
              <h2>{copy.releaseCheck}</h2>
            </div>
            <span
              className={`status-chip ${
                releaseState.latest?.updateAvailable ? 'warm' : ''
              }`}
            >
              {releaseState.latest
                ? releaseState.latest.updateAvailable
                  ? copy.updateAvailable
                  : copy.current
                : copy.notChecked}
            </span>
          </summary>

          <div className="control-body">
            <p className="control-copy">
              {copy.releaseCopy}
            </p>

            <label className="field">
              <span>{copy.repository}</span>
              <input
                value={snapshot.updates.repositorySlug}
                onChange={(event) =>
                  patchUpdates({ repositorySlug: event.target.value })
                }
                placeholder="owner/repo"
              />
            </label>

            <label className="field">
              <span>{copy.releasesPage}</span>
              <input
                value={snapshot.updates.releasesUrl}
                onChange={(event) => patchUpdates({ releasesUrl: event.target.value })}
                placeholder="https://github.com/owner/repo/releases"
              />
            </label>

            <div className="token-stack">
              <div className="token-row">
                <span>{copy.installed}</span>
                <span>{snapshot.updates.currentVersion}</span>
              </div>
              <div className="token-row">
                <span>{copy.latestCheckApi}</span>
                <span className="truncate">{snapshot.updates.latestReleaseApiUrl}</span>
              </div>
            </div>

            {releaseState.error ? (
              <p className="inline-note error">{releaseState.error}</p>
            ) : releaseState.latest ? (
              <div className="release-panel">
                <div className="release-head">
                  <div>
                    <p className="theme-name">
                      {releaseState.latest.name || releaseState.latest.tagName}
                    </p>
                    <p className="theme-summary">
                      {copy.published}{' '}
                      {formatDate(
                        releaseState.latest.publishedAt,
                        snapshot.language,
                        snapshot.resolvedLanguage,
                        copy.unknownDate,
                      )}
                    </p>
                  </div>
                  {releaseState.latest.prerelease ? (
                    <span className="mode-chip">{copy.prerelease}</span>
                  ) : null}
                </div>

                <p className="release-body">
                  {releaseState.latest.body || copy.noReleaseNotes}
                </p>

                {releaseState.latest.assets.length > 0 ? (
                  <div className="asset-list">
                    {releaseState.latest.assets.slice(0, 3).map((asset) => (
                      <button
                        key={asset.browserDownloadUrl}
                        className="asset-link"
                        onClick={() => handleOpenExternal(asset.browserDownloadUrl)}
                      >
                        {asset.name}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : (
              <p className="inline-note">
                {copy.noReleaseCheck}
              </p>
            )}

            <div className="button-row">
              <button
                className="theme-button"
                onClick={handleUpdateSettingsSave}
                disabled={busyAction === 'save-updates'}
              >
                {busyAction === 'save-updates'
                  ? copy.savingUpdateSettings
                  : copy.saveUpdateSettings}
              </button>
              <button
                className="theme-button"
                onClick={handleReleaseCheck}
                disabled={releaseState.loading}
              >
                {releaseState.loading ? copy.checkingRelease : copy.checkGitHubRelease}
              </button>
              <button
                className="theme-button muted-button"
                onClick={() => handleOpenExternal(snapshot.updates.releasesUrl)}
                disabled={busyAction === `open:${snapshot.updates.releasesUrl}`}
              >
                {copy.openReleases}
              </button>
            </div>
          </div>
        </details>
      </section>
    </main>
  )
}

function buildShellStyle(theme: ThemeDefinition) {
  if (theme.mode === 'system') {
    return {
      '--accent': theme.accent,
    } as CSSProperties
  }

  return {
    '--canvas': theme.canvas,
    '--surface': theme.surface,
    '--surface-strong': theme.surfaceStrong,
    '--line': theme.line,
    '--text': theme.text,
    '--muted': theme.muted,
    '--accent': theme.accent,
    '--accent-soft': theme.accentSoft,
  } as CSSProperties
}

function buildThemeOptionStyle(theme: ThemeDefinition) {
  return {
    '--option-canvas': theme.canvas,
    '--option-surface': theme.surface,
    '--option-surface-strong': theme.surfaceStrong,
    '--option-line': theme.line,
    '--option-text': theme.text,
    '--option-muted': theme.muted,
    '--option-accent': theme.accent,
  } as CSSProperties
}

function getThemeDisplay(
  theme: ThemeDefinition,
  language: BootstrapPayload['language'],
  resolvedLanguage: BootstrapPayload['resolvedLanguage'] = 'en-US',
) {
  if (getResolvedLanguage(language, resolvedLanguage) !== 'zh-CN') {
    return {
      name: theme.name,
      summary: theme.summary,
    }
  }

  const zhCopy = ZH_THEME_COPY[theme.id]
  return {
    name: theme.id === 'custom' ? theme.name || zhCopy?.name || '自定义' : zhCopy?.name ?? theme.name,
    summary: zhCopy?.summary ?? theme.summary,
  }
}

function getCopy(
  language: BootstrapPayload['language'],
  resolvedLanguage: BootstrapPayload['resolvedLanguage'] = 'en-US',
) {
  return COPY[getResolvedLanguage(language, resolvedLanguage)]
}

function getResolvedLanguage(
  language: BootstrapPayload['language'],
  resolvedLanguage: BootstrapPayload['resolvedLanguage'] = 'en-US',
): keyof typeof COPY {
  if (language === 'zh-CN' || language === 'en-US') {
    return language
  }

  if (resolvedLanguage === 'zh-CN' || resolvedLanguage === 'en-US') {
    return resolvedLanguage
  }

  const systemLanguage = typeof navigator === 'undefined' ? 'en-US' : navigator.language
  return systemLanguage.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US'
}

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : 'Unknown desktop error'
}

function splitArgs(value: string) {
  return value
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
}

function normalizeVersion(value: string) {
  return value
    .trim()
    .replace(/^v/i, '')
    .split('-')[0]
}

function compareVersions(left: string, right: string) {
  const leftParts = normalizeVersion(left)
    .split('.')
    .map((part) => Number.parseInt(part, 10) || 0)
  const rightParts = normalizeVersion(right)
    .split('.')
    .map((part) => Number.parseInt(part, 10) || 0)
  const max = Math.max(leftParts.length, rightParts.length)

  for (let index = 0; index < max; index += 1) {
    const l = leftParts[index] ?? 0
    const r = rightParts[index] ?? 0

    if (l > r) {
      return 1
    }

    if (l < r) {
      return -1
    }
  }

  return 0
}

function mapReleasePayload(payload: unknown, currentVersion: string): ReleaseInfo {
  const release = payload as {
    tag_name?: string
    name?: string
    html_url?: string
    published_at?: string
    body?: string
    prerelease?: boolean
    assets?: Array<{ name?: string; browser_download_url?: string }>
  }

  const tagName = release.tag_name ?? 'unknown'
  return {
    tagName,
    name: release.name ?? '',
    htmlUrl: release.html_url ?? '',
    publishedAt: release.published_at ?? '',
    body: release.body ?? '',
    prerelease: Boolean(release.prerelease),
    assets: (release.assets ?? [])
      .filter((asset) => asset.browser_download_url)
      .map((asset) => ({
        name: asset.name ?? 'download',
        browserDownloadUrl: asset.browser_download_url ?? '',
      })),
    updateAvailable: compareVersions(tagName, currentVersion) > 0,
  }
}

function formatDate(
  value: string,
  language: BootstrapPayload['language'],
  resolvedLanguage: BootstrapPayload['resolvedLanguage'],
  fallback: string,
) {
  if (!value) {
    return fallback
  }

  return new Intl.DateTimeFormat(getResolvedLanguage(language, resolvedLanguage), {
    dateStyle: 'medium',
  }).format(new Date(value))
}

export default App
