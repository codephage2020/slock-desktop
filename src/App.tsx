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
  saveServiceSettings,
  saveUpdateSettings,
  startService,
  stopService,
  updateTheme,
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

  if (!snapshot) {
    return (
      <main className="loading-shell">
        <p className="eyebrow">SLOCK DESKTOP</p>
        <h1>Theme Studio is booting.</h1>
        <p>Preparing the local shell and reading your desktop preferences.</p>
      </main>
    )
  }

  const activeTheme =
    snapshot.themes.find((theme) => theme.id === snapshot.activeThemeId) ??
    snapshot.themes[0]

  const shellStyle = buildShellStyle(activeTheme)
  const stackButtonLabel =
    snapshot.service.autoStartWithWorkspace && snapshot.service.configured
      ? 'Launch Stack'
      : snapshot.workspaceOpen
        ? 'Focus Workspace'
        : 'Launch Workspace'

  return (
    <main className="studio-shell" data-mode={activeTheme.mode} style={shellStyle}>
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      <header className="masthead">
        <p className="eyebrow">{snapshot.appName}</p>
        <p className="eyebrow subtle">Theme Studio / Phase 2</p>
      </header>

      <section className="hero-grid">
        <div className="hero-copy">
          <p className="kicker">Desktop control surface</p>
          <h1>Operate the workspace, the local service, and the release line.</h1>
          <p className="lede">
            The desktop shell now owns runtime theming, local stack launch
            policy, and GitHub release awareness from one control plane.
          </p>
        </div>

        <aside className="workspace-panel">
          <div className="status-row">
            <span className="status-dot" />
            <span>{snapshot.workspaceOpen ? 'Workspace active' : 'Workspace parked'}</span>
          </div>

          <dl className="meta-list">
            <div>
              <dt>Target</dt>
              <dd>{snapshot.workspaceUrl}</dd>
            </div>
            <div>
              <dt>Local service</dt>
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
          <p className="settings-sidebar-title">Settings</p>
          <button className="settings-nav-item active" type="button">
            <span className="settings-nav-icon">A</span>
            <span>Appearance</span>
          </button>
          <button className="settings-nav-item" type="button">
            <span className="settings-nav-icon">S</span>
            <span>Service</span>
          </button>
          <button className="settings-nav-item" type="button">
            <span className="settings-nav-icon">U</span>
            <span>Updates</span>
          </button>
        </aside>

        <div className="settings-content">
          <div className="settings-title-row">
            <div>
              <p className="eyebrow">Desktop Settings</p>
              <h2 id="appearance-settings-title">Appearance</h2>
              <p className="settings-description">
                Theme settings now behave like a real settings surface: choose a
                mode, preview the workspace, and apply it immediately.
              </p>
            </div>
            <span className="settings-save-state">Saved locally</span>
          </div>

          <div className="setting-row">
            <div className="setting-copy">
              <p className="setting-label">Theme</p>
              <p>Sync the desktop shell and remote workspace overlay.</p>
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

          <div className="setting-row compact">
            <div className="setting-copy">
              <p className="setting-label">Apply scope</p>
              <p>Current theme covers the local shell, page background, cards, inputs, messages, and popovers.</p>
            </div>
            <span className="scope-pill">{activeTheme.mode} mode</span>
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
        <article className="control-card">
          <div className="control-card-head">
            <div>
              <p className="eyebrow">Local Service</p>
              <h2>One-click stack startup</h2>
            </div>
            <span className={`status-chip ${snapshot.service.running ? 'live' : ''}`}>
              {snapshot.service.running ? 'running' : 'idle'}
            </span>
          </div>

          <p className="control-copy">
            Keep a local daemon or API beside the desktop shell. The workspace
            launcher can start it automatically before bringing the app forward.
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
                ? 'The service command is saved locally in your app config directory.'
                : 'Leave the command empty if this desktop build should open the cloud workspace only.'}
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
        </article>

        <article className="control-card">
          <div className="control-card-head">
            <div>
              <p className="eyebrow">Update Center</p>
              <h2>GitHub release awareness</h2>
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
          </div>

          <p className="control-copy">
            This stage checks the GitHub release channel and opens the release
            page in one click. Signed in-app self-update is the next step.
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
              No release check yet. Run a GitHub check after saving the release
              channel.
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
        </article>
      </section>

      <section className="roadmap-strip">
        <p className="eyebrow">Queued next</p>
        <div className="pill-row">
          <span className="pill">Signed updater</span>
          <span className="pill">Service health checks</span>
          <span className="pill">Release workflow</span>
          <span className="pill">OS autostart</span>
        </div>
      </section>
    </main>
  )
}

function buildShellStyle(theme: ThemeDefinition) {
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
