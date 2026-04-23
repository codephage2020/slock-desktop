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
  { id: 'light', label: 'Light' },
  { id: 'dark', label: 'Dark' },
  { id: 'system', label: 'System' },
] as const

const LANGUAGE_OPTIONS = [
  { id: 'en-US', label: 'English' },
  { id: 'zh-CN', label: '中文' },
  { id: 'system', label: 'System' },
] as const

const COPY = {
  'en-US': {
    header: 'Desktop Console',
    kicker: 'Desktop control surface',
    title: 'Slock workspace launcher',
    lede:
      'Open the original Slock workspace, choose a theme color, and keep the desktop shell aligned with light, dark, or system mode.',
    workspaceActive: 'Workspace active',
    workspaceParked: 'Workspace parked',
    target: 'Target',
    localService: 'Local service',
    settings: 'Settings',
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
    language: 'Language',
    languageDescription: 'Choose Chinese, English, or follow the operating system.',
    applyScope: 'Apply scope',
    applyDescription:
      'Current theme covers startup page, settings, workspace overlay, and remote page injection.',
    savedLocally: 'Saved locally',
    saveCustomTheme: 'Save Custom Theme',
    saving: 'Saving…',
    modeSuffix: 'mode',
  },
  'zh-CN': {
    header: '桌面控制台',
    kicker: '桌面控制面板',
    title: 'Slock 工作区启动器',
    lede: '打开原始 Slock 工作区，选择主题色，并让桌面壳在亮色、暗黑或跟随系统模式下保持一致。',
    workspaceActive: '工作区已打开',
    workspaceParked: '工作区待启动',
    target: '目标地址',
    localService: '本地服务',
    settings: '设置',
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
    customThemeDescription: '定义个人强调色，并保存为 Custom 主题。',
    language: '语言',
    languageDescription: '选择中文、英文，或跟随操作系统。',
    applyScope: '应用范围',
    applyDescription: '当前主题覆盖起始页、设置页、工作区浮层和远端页面注入。',
    savedLocally: '已保存本地',
    saveCustomTheme: '保存自定义主题',
    saving: '保存中…',
    modeSuffix: '模式',
  },
} as const

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
    themeMode: BootstrapPayload['activeThemeMode'],
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
    language: BootstrapPayload['activeLanguage'],
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
    return (
      <main className="loading-shell">
        <p className="eyebrow">SLOCK DESKTOP</p>
        <h1>Desktop Console is booting.</h1>
        <p>Preparing the local shell and reading your desktop preferences.</p>
      </main>
    )
  }

  const activeTheme =
    snapshot.themes.find((theme) => theme.id === snapshot.activeThemeId) ??
    snapshot.themes[0]
  const copy = getCopy(snapshot.activeLanguage)

  const shellStyle = buildShellStyle(activeTheme)
  const stackButtonLabel =
    snapshot.service.autoStartWithWorkspace && snapshot.service.configured
      ? 'Launch Stack'
      : snapshot.workspaceOpen
        ? 'Focus Workspace'
        : 'Open Workspace Here'

  return (
    <main className="studio-shell" data-mode={activeTheme.mode} style={shellStyle}>
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      <header className="masthead">
        <p className="eyebrow">{snapshot.appName}</p>
        <p className="eyebrow subtle">{copy.header}</p>
      </header>

      <section className="hero-grid">
        <div className="hero-copy">
          <p className="kicker">{copy.kicker}</p>
          <h1>{copy.title}</h1>
          <p className="lede">{copy.lede}</p>
        </div>

        <aside className="workspace-panel">
          <div className="status-row">
            <span className="status-dot" />
            <span>{snapshot.workspaceOpen ? copy.workspaceActive : copy.workspaceParked}</span>
          </div>

          <dl className="meta-list">
            <div>
              <dt>{copy.target}</dt>
              <dd>{snapshot.workspaceUrl}</dd>
            </div>
            <div>
              <dt>{copy.localService}</dt>
              <dd>
                {snapshot.service.running
                  ? `Running${snapshot.service.pid ? ` / PID ${snapshot.service.pid}` : ''}`
                  : snapshot.service.configured
                    ? 'Configured / idle'
                    : 'Not configured'}
              </dd>
            </div>
          </dl>

          <button
            className="launch-button"
            onClick={handleWorkspaceOpen}
            disabled={busyAction === 'workspace'}
          >
            {busyAction === 'workspace' ? 'Launching…' : stackButtonLabel}
          </button>
        </aside>
      </section>

      {errorMessage ? (
        <section className="error-banner" role="alert">
          <strong>Desktop state error</strong>
          <p>{errorMessage}</p>
        </section>
      ) : null}

      <section className="settings-shell" aria-labelledby="appearance-settings-title">
        <aside className="settings-sidebar" aria-label="Desktop settings sections">
          <p className="settings-sidebar-title">{copy.settings}</p>
          <button className="settings-nav-item active" type="button">
            <span className="settings-nav-icon">A</span>
            <span>{copy.appearance}</span>
          </button>
          <button className="settings-nav-item" type="button">
            <span className="settings-nav-icon">S</span>
            <span>{copy.service}</span>
          </button>
          <button className="settings-nav-item" type="button">
            <span className="settings-nav-icon">U</span>
            <span>{copy.updates}</span>
          </button>
        </aside>

        <div className="settings-content">
          <div className="settings-title-row">
            <div>
              <p className="eyebrow">{copy.desktopSettings}</p>
              <h2 id="appearance-settings-title">{copy.appearance}</h2>
              <p className="settings-description">{copy.appearanceDescription}</p>
            </div>
            <span className="settings-save-state">{copy.savedLocally}</span>
          </div>

          <div className="setting-row compact">
            <div className="setting-copy">
              <p className="setting-label">{copy.mode}</p>
              <p>{copy.modeDescription}</p>
            </div>

            <div className="mode-picker" role="radiogroup" aria-label="Theme mode">
              {THEME_MODES.map((mode) => {
                const selected = mode.id === snapshot.activeThemeMode
                return (
                  <button
                    key={mode.id}
                    className={`mode-option${selected ? ' selected' : ''}`}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() => handleThemeModeChange(mode.id)}
                    disabled={busyAction === `mode:${mode.id}`}
                  >
                    {mode.label}
                  </button>
                )
              })}
            </div>
          </div>

          <div className="setting-row">
            <div className="setting-copy">
              <p className="setting-label">{copy.themeColor}</p>
              <p>{copy.themeDescription}</p>
            </div>

            <div className="theme-picker" role="radiogroup" aria-label="Theme">
              {snapshot.themes.map((theme) => {
                const selected = theme.id === snapshot.activeThemeId
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
                      <span className="theme-option-name">{theme.name}</span>
                      <span className="theme-option-summary">{theme.summary}</span>
                    </span>
                    <span className="theme-option-check" aria-hidden="true">
                      {selected ? '✓' : ''}
                    </span>
                  </button>
                )
              })}
            </div>
          </div>

          <div className="setting-row custom-theme-row">
            <div className="setting-copy">
              <p className="setting-label">{copy.customTheme}</p>
              <p>{copy.customThemeDescription}</p>
            </div>

            <div className="custom-theme-controls">
              <label className="field compact-field">
                <span>Name</span>
                <input
                  value={snapshot.customTheme.name}
                  onChange={(event) =>
                    patchCustomTheme({ name: event.target.value })
                  }
                  placeholder="Custom"
                />
              </label>

              <label className="field compact-field color-field">
                <span>Accent</span>
                <input
                  type="color"
                  value={snapshot.customTheme.accent}
                  onChange={(event) =>
                    patchCustomTheme({ accent: event.target.value })
                  }
                  aria-label="Custom theme accent color"
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

          <div className="setting-row compact">
            <div className="setting-copy">
              <p className="setting-label">{copy.language}</p>
              <p>{copy.languageDescription}</p>
            </div>

            <div className="mode-picker" role="radiogroup" aria-label="Language">
              {LANGUAGE_OPTIONS.map((language) => {
                const selected = language.id === snapshot.activeLanguage
                return (
                  <button
                    key={language.id}
                    className={`mode-option${selected ? ' selected' : ''}`}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() => handleLanguageChange(language.id)}
                    disabled={busyAction === `language:${language.id}`}
                  >
                    {language.label}
                  </button>
                )
              })}
            </div>
          </div>

          <div className="setting-row compact">
            <div className="setting-copy">
              <p className="setting-label">{copy.applyScope}</p>
              <p>{copy.applyDescription}</p>
            </div>
            <span className="scope-pill">
              {snapshot.activeThemeMode} {copy.modeSuffix}
            </span>
          </div>
        </div>

        <aside className="appearance-preview" aria-label={`${activeTheme.name} preview`}>
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
                <p>Theme settings should feel native.</p>
              </div>
              <div className="preview-message assistant">
                <span />
                <p>{activeTheme.name} keeps the workspace quiet and readable.</p>
              </div>
              <div className="preview-composer">
                <span>Previewing {activeTheme.name}</span>
                <button type="button" aria-label="Preview send button">↵</button>
              </div>
            </div>
          </div>
        </aside>
      </section>

      <section className="operations-grid">
        <details className="control-card compact-control">
          <summary className="control-card-head">
            <div>
              <p className="eyebrow">Local Service</p>
              <h2>Service startup</h2>
            </div>
            <span className={`status-chip ${snapshot.service.running ? 'live' : ''}`}>
              {snapshot.service.running ? 'running' : 'idle'}
            </span>
          </summary>

          <div className="control-body">
            <p className="control-copy">
              Optional local service command for workspace launch.
            </p>

            <label className="field">
              <span>Command path</span>
              <input
                value={snapshot.service.commandPath}
                onChange={(event) =>
                  patchService({ commandPath: event.target.value })
                }
                placeholder="/absolute/path/to/service"
              />
            </label>

            <label className="field">
              <span>Working directory</span>
              <input
                value={snapshot.service.workingDirectory}
                onChange={(event) =>
                  patchService({ workingDirectory: event.target.value })
                }
                placeholder="/absolute/path/to/project"
              />
            </label>

            <label className="field">
              <span>Arguments</span>
              <textarea
                value={snapshot.service.args.join('\n')}
                onChange={(event) =>
                  patchService({ args: splitArgs(event.target.value) })
                }
                placeholder={'One argument per line\n--port\n3141'}
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
              <span>Auto-start the service when launching the workspace</span>
            </label>

            {snapshot.service.lastError ? (
              <p className="inline-note error">{snapshot.service.lastError}</p>
            ) : (
              <p className="inline-note">
                {snapshot.service.configured
                  ? 'Service command saved locally.'
                  : 'Leave empty for cloud workspace only.'}
              </p>
            )}

            <div className="button-row">
              <button
                className="theme-button"
                onClick={handleServiceSave}
                disabled={busyAction === 'save-service'}
              >
                {busyAction === 'save-service' ? 'Saving…' : 'Save Service Settings'}
              </button>
              <button
                className="theme-button"
                onClick={handleServiceStart}
                disabled={busyAction === 'start-service'}
              >
                {busyAction === 'start-service' ? 'Starting…' : 'Start Service'}
              </button>
              <button
                className="theme-button muted-button"
                onClick={handleServiceStop}
                disabled={busyAction === 'stop-service' || !snapshot.service.running}
              >
                {busyAction === 'stop-service' ? 'Stopping…' : 'Stop Service'}
              </button>
            </div>
          </div>
        </details>

        <details className="control-card compact-control">
          <summary className="control-card-head">
            <div>
              <p className="eyebrow">Update Center</p>
              <h2>Release check</h2>
            </div>
            <span
              className={`status-chip ${
                releaseState.latest?.updateAvailable ? 'warm' : ''
              }`}
            >
              {releaseState.latest
                ? releaseState.latest.updateAvailable
                  ? 'update available'
                  : 'current'
                : 'not checked'}
            </span>
          </summary>

          <div className="control-body">
            <p className="control-copy">
              Check the configured GitHub release channel.
            </p>

            <label className="field">
              <span>Repository</span>
              <input
                value={snapshot.updates.repositorySlug}
                onChange={(event) =>
                  patchUpdates({ repositorySlug: event.target.value })
                }
                placeholder="owner/repo"
              />
            </label>

            <label className="field">
              <span>Releases page</span>
              <input
                value={snapshot.updates.releasesUrl}
                onChange={(event) => patchUpdates({ releasesUrl: event.target.value })}
                placeholder="https://github.com/owner/repo/releases"
              />
            </label>

            <div className="token-stack">
              <div className="token-row">
                <span>Installed</span>
                <span>{snapshot.updates.currentVersion}</span>
              </div>
              <div className="token-row">
                <span>Latest check API</span>
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
                      Published {formatDate(releaseState.latest.publishedAt)}
                    </p>
                  </div>
                  {releaseState.latest.prerelease ? (
                    <span className="mode-chip">prerelease</span>
                  ) : null}
                </div>

                <p className="release-body">
                  {releaseState.latest.body || 'No release notes were provided for this release.'}
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
                No release check yet.
              </p>
            )}

            <div className="button-row">
              <button
                className="theme-button"
                onClick={handleUpdateSettingsSave}
                disabled={busyAction === 'save-updates'}
              >
                {busyAction === 'save-updates' ? 'Saving…' : 'Save Update Settings'}
              </button>
              <button
                className="theme-button"
                onClick={handleReleaseCheck}
                disabled={releaseState.loading}
              >
                {releaseState.loading ? 'Checking…' : 'Check GitHub Release'}
              </button>
              <button
                className="theme-button muted-button"
                onClick={() => handleOpenExternal(snapshot.updates.releasesUrl)}
                disabled={busyAction === `open:${snapshot.updates.releasesUrl}`}
              >
                Open Releases
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

function getCopy(language: BootstrapPayload['activeLanguage']) {
  if (language === 'zh-CN' || language === 'en-US') {
    return COPY[language]
  }

  const systemLanguage =
    typeof navigator === 'undefined' ? 'en-US' : navigator.language

  return systemLanguage.toLowerCase().startsWith('zh')
    ? COPY['zh-CN']
    : COPY['en-US']
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

function formatDate(value: string) {
  if (!value) {
    return 'unknown date'
  }

  return new Intl.DateTimeFormat('en', {
    dateStyle: 'medium',
  }).format(new Date(value))
}

export default App
