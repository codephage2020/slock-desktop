import { type CSSProperties, startTransition, useEffect, useState } from 'react'
import './App.css'
import {
  type BootstrapPayload,
  type ThemeDefinition,
  loadBootstrap,
  openWorkspace,
  updateTheme,
} from './lib/desktop'

function App() {
  const [snapshot, setSnapshot] = useState<BootstrapPayload | null>(null)
  const [busyAction, setBusyAction] = useState<string | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

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

  return (
    <main className="studio-shell" data-mode={activeTheme.mode} style={shellStyle}>
      <div className="ambient ambient-left" />
      <div className="ambient ambient-right" />

      <header className="masthead">
        <p className="eyebrow">{snapshot.appName}</p>
        <p className="eyebrow subtle">Theme Studio / Phase 1</p>
      </header>

      <section className="hero-grid">
        <div className="hero-copy">
          <p className="kicker">Desktop skin system</p>
          <h1>Shape the shell before the launcher and updater arrive.</h1>
          <p className="lede">
            This control surface owns the local visual system. The workspace
            window inherits the active theme each time it opens or reloads.
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
              <dt>Live theme</dt>
              <dd>{activeTheme.name}</dd>
            </div>
          </dl>

          <button
            className="launch-button"
            onClick={handleWorkspaceOpen}
            disabled={busyAction === 'workspace'}
          >
            {busyAction === 'workspace'
              ? 'Opening workspace…'
              : snapshot.workspaceOpen
                ? 'Focus Workspace'
                : 'Launch Workspace'}
          </button>
        </aside>
      </section>

      {errorMessage ? (
        <section className="error-banner" role="alert">
          <strong>Desktop state error</strong>
          <p>{errorMessage}</p>
        </section>
      ) : null}

      <section className="theme-headline">
        <div>
          <p className="eyebrow">Theme catalog</p>
          <h2>Three stable palettes. One stored choice.</h2>
        </div>
        <p className="theme-note">
          The shell stays consistent locally. The remote Slock workspace gets a
          matching overlay through injected theme tokens.
        </p>
      </section>

      <section className="theme-grid">
        {snapshot.themes.map((theme) => {
          const selected = theme.id === snapshot.activeThemeId
          return (
            <article
              key={theme.id}
              className={`theme-card${selected ? ' selected' : ''}`}
            >
              <div className="swatch-rail" aria-hidden="true">
                {theme.preview.map((color) => (
                  <span
                    key={`${theme.id}-${color}`}
                    className="swatch"
                    style={{ background: color }}
                  />
                ))}
              </div>

              <div className="theme-card-body">
                <div className="theme-card-header">
                  <div>
                    <p className="theme-name">{theme.name}</p>
                    <p className="theme-summary">{theme.summary}</p>
                  </div>
                  <span className="mode-chip">{theme.mode}</span>
                </div>

                <div className="token-row">
                  <span>Canvas</span>
                  <span>{theme.canvas}</span>
                </div>
                <div className="token-row">
                  <span>Accent</span>
                  <span>{theme.accent}</span>
                </div>

                <button
                  className="theme-button"
                  onClick={() => handleThemeChange(theme.id)}
                  disabled={busyAction === theme.id}
                >
                  {selected
                    ? 'Active Theme'
                    : busyAction === theme.id
                      ? 'Applying…'
                      : 'Apply Theme'}
                </button>
              </div>
            </article>
          )
        })}
      </section>

      <section className="roadmap-strip">
        <p className="eyebrow">Queued next</p>
        <div className="pill-row">
          <span className="pill">Workspace launcher</span>
          <span className="pill">Sidecar service</span>
          <span className="pill">In-app updater</span>
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

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : 'Unknown desktop error'
}

export default App
