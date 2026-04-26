import { type CSSProperties, startTransition, useEffect, useRef, useState } from 'react'
import './App.css'
import './Settings.css'
import {
  type BootstrapPayload,
  type CustomThemeSnapshot,
  type ThemeDefinition,
  createCustomTheme,
  deleteCustomTheme,
  installDesktopUpdate,
  loadBootstrap,
  openWorkspace,
  refreshServiceServerCatalog,
  refreshServiceServers,
  renameCustomTheme,
  selectServiceServer,
  startService,
  stopService,
  updateCustomThemeAccent,
  updateLanguage,
  updateTheme,
  updateThemeMode,
} from './lib/desktop'

interface ReleaseInfo {
  tagName: string
  name: string
  publishedAt: string
  body: string
  prerelease: boolean
  updateAvailable: boolean
}

interface ReleaseState {
  loading: boolean
  installing: boolean
  error: string | null
  latest: ReleaseInfo | null
}

const INITIAL_RELEASE_STATE: ReleaseState = {
  loading: false,
  installing: false,
  error: null,
  latest: null,
}

const ORIGINAL_SWATCH = '#ffd701'
const DEFAULT_NEW_THEME_ACCENT = '#10a37f'

const THEME_MODES = [
  { id: 'light', icon: 'sun', labelKey: 'modeLight' },
  { id: 'dark', icon: 'moon', labelKey: 'modeDark' },
  { id: 'system', icon: 'display', labelKey: 'modeSystem' },
] as const

const LANGUAGE_OPTIONS = [
  { id: 'en-US', icon: 'latin', labelKey: 'languageEnglish' },
  { id: 'zh-CN', icon: 'han', labelKey: 'languageChinese' },
  { id: 'system', icon: 'globe', labelKey: 'languageSystem' },
] as const

const COPY = {
  'en-US': {
    workspaceActive: 'Workspace active',
    workspaceParked: 'Workspace parked',
    appearance: 'Theme',
    service: 'Server',
    updates: 'Updates',
    mode: 'Mode',
    themeColor: 'Theme color',
    customTheme: 'My accent',
    customThemeAccent: 'Accent',
    customThemeAccentAria: 'Personal accent color',
    customThemeNamePlaceholder: 'Untitled theme',
    language: 'Language',
    saving: 'Saving…',
    modeLight: 'Light',
    modeDark: 'Dark',
    modeSystem: 'System',
    languageEnglish: 'English',
    languageChinese: 'Chinese',
    languageSystem: 'System',
    focusSlock: 'Focus Slock',
    openSlock: 'Enter Slock',
    launching: 'Launching…',
    launchingTitle: 'Opening Slock',
    launchingDetail: 'Preparing workspace',
    running: 'Running',
    configuredIdle: 'Configured / not running',
    notConfigured: 'Not configured',
    desktopStateError: 'Desktop state error',
    serviceRunning: 'running',
    serviceIdle: 'not running',
    serviceOffline: 'not running',
    serviceNotLinked: 'no local binding',
    serviceSignInRequired: 'sign in required',
    serviceCopy: 'Desktop reads your server list from the signed-in Slock session and starts the selected server in the background when it is not running.',
    serverSearch: 'Find server',
    noMatchingServers: 'No matching servers.',
    selectedServer: 'Server',
    selectedServerPlaceholder: 'Choose a server',
    noServers: 'No servers available on this account yet.',
    refreshServers: 'Refresh Servers',
    refreshingServers: 'Refreshing…',
    loadingServerCatalog: 'Loading server list…',
    syncingServerStatus: 'Checking local server status…',
    startingSelectedServer: 'Starting selected server…',
    closingSelectedServer: 'Closing selected server…',
    savingSelectedServer: 'Saving selected server…',
    serviceSignInHint: 'Open Slock once, sign in, and the launcher will sync your server list automatically.',
    machineStatus: 'Machine status',
    startService: 'Start Service',
    startingService: 'Starting…',
    stopService: 'Stop Service',
    stoppingService: 'Stopping…',
    closeServer: 'Close server',
    closingServer: 'Closing…',
    serviceNotRunning: 'Selected server service is not running.',
    updateService: 'Update Daemon',
    updatingService: 'Updating…',
    releaseCheck: 'Version',
    updateAvailable: 'update available',
    current: 'up to date',
    notChecked: 'not checked',
    currentVersion: 'Current version',
    prerelease: 'prerelease',
    published: 'Published',
    noReleaseNotes: 'No release notes were provided for this release.',
    checkGitHubRelease: 'Check for Updates',
    checkingRelease: 'Checking…',
    installDesktopUpdate: 'Update',
    installingDesktopUpdate: 'Updating…',
    unknownDate: 'unknown date',
    themeOriginalName: 'Original',
    themeOriginalSummary: 'Slock’s native appearance.',
    themeNewLabel: 'New theme',
    themeRename: 'Rename',
    themeDelete: 'Delete',
    themeBuiltIn: 'Built-in',
    themeEmptyHint: 'No custom themes yet — tap + to create one.',
    themeRenameSave: 'Save',
    themeRenameCancel: 'Cancel',
    themeNewTitle: 'Create theme',
    themeCreate: 'Create',
    creatingTheme: 'Creating…',
    deletingTheme: 'Deleting…',
    appBootingTitle: 'slock.ai',
    appBootingDetail: 'Starting desktop…',
  },
  'zh-CN': {
    workspaceActive: '工作区已打开',
    workspaceParked: '工作区待启动',
    appearance: '主题',
    service: '服务',
    updates: '更新',
    mode: '模式',
    themeColor: '主题色彩',
    customTheme: '我的强调色',
    customThemeAccent: '强调色',
    customThemeAccentAria: '我的强调色',
    customThemeNamePlaceholder: '未命名主题',
    language: '语言',
    saving: '保存中…',
    modeLight: '亮色',
    modeDark: '暗黑',
    modeSystem: '系统',
    languageEnglish: '英文',
    languageChinese: '中文',
    languageSystem: '系统',
    focusSlock: '聚焦 Slock',
    openSlock: '进入 Slock',
    launching: '启动中…',
    launchingTitle: '正在进入 Slock',
    launchingDetail: '正在准备工作区',
    running: '运行中',
    configuredIdle: '已配置 / 未运行',
    notConfigured: '未配置',
    desktopStateError: '桌面状态错误',
    serviceRunning: '运行中',
    serviceIdle: '未运行',
    serviceOffline: '未运行',
    serviceNotLinked: '未创建本地绑定',
    serviceSignInRequired: '需要登录',
    serviceCopy: '桌面端会从已登录的 Slock 会话读取 server 列表；所选 server 未运行时，会在后台自动拉起对应 daemon。',
    serverSearch: '搜索 server',
    noMatchingServers: '没有匹配的 server。',
    selectedServer: 'Server',
    selectedServerPlaceholder: '选择一个 server',
    noServers: '当前账号下还没有可用 server。',
    refreshServers: '刷新 Server 列表',
    refreshingServers: '刷新中…',
    loadingServerCatalog: '正在读取 Server 列表…',
    syncingServerStatus: '正在同步本地 Server 状态…',
    startingSelectedServer: '正在启动所选 Server…',
    closingSelectedServer: '正在关闭所选 Server…',
    savingSelectedServer: '正在保存所选 Server…',
    serviceSignInHint: '先打开一次 Slock 并完成登录，launcher 就会自动同步 server 列表。',
    machineStatus: '本地 machine 状态',
    startService: '启动服务',
    startingService: '启动中…',
    stopService: '停止服务',
    stoppingService: '停止中…',
    closeServer: '关闭 Server',
    closingServer: '关闭中…',
    serviceNotRunning: '所选 server 服务未运行。',
    updateService: '更新 Daemon',
    updatingService: '更新中…',
    releaseCheck: '版本',
    updateAvailable: '有可用更新',
    current: '已是最新',
    notChecked: '未检查',
    currentVersion: '当前版本',
    prerelease: '预发布',
    published: '发布于',
    noReleaseNotes: '此版本没有提供发布说明。',
    checkGitHubRelease: '检查更新',
    checkingRelease: '检查中…',
    installDesktopUpdate: '更新',
    installingDesktopUpdate: '更新中…',
    unknownDate: '未知日期',
    themeOriginalName: '原主题',
    themeOriginalSummary: '保持 Slock 原生外观。',
    themeNewLabel: '新建主题',
    themeRename: '重命名',
    themeDelete: '删除',
    themeBuiltIn: '内置',
    themeEmptyHint: '还没有自定义主题，点击 + 新建。',
    themeRenameSave: '保存',
    themeRenameCancel: '取消',
    themeNewTitle: '新建主题',
    themeCreate: '创建',
    creatingTheme: '创建中…',
    deletingTheme: '删除中…',
    appBootingTitle: 'slock.ai',
    appBootingDetail: '正在启动桌面端…',
  },
} as const

type UiCopy = (typeof COPY)[keyof typeof COPY]
type ServiceRefreshPhase = 'catalog' | 'status' | null

interface NewThemeDraft {
  name: string
  accent: string
}

function App() {
  const [snapshot, setSnapshot] = useState<BootstrapPayload | null>(null)
  const [busyAction, setBusyAction] = useState<string | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [releaseState, setReleaseState] = useState<ReleaseState>(INITIAL_RELEASE_STATE)
  const [serverQuery, setServerQuery] = useState('')
  const [workspaceLaunchActive, setWorkspaceLaunchActive] = useState(false)
  const [workspaceLaunchTarget, setWorkspaceLaunchTarget] = useState<string | null>(null)
  const [serviceRefreshPhase, setServiceRefreshPhase] = useState<ServiceRefreshPhase>(null)
  const [renamingThemeId, setRenamingThemeId] = useState<string | null>(null)
  const [renameDraft, setRenameDraft] = useState('')
  const [newThemeDraft, setNewThemeDraft] = useState<NewThemeDraft | null>(null)
  const renameInputRef = useRef<HTMLInputElement | null>(null)
  const newNameInputRef = useRef<HTMLInputElement | null>(null)
  const initialServiceRefreshRef = useRef(false)
  const savedServiceSlugRef = useRef('')

  useEffect(() => {
    let cancelled = false

    void loadBootstrap(false)
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

  useEffect(() => {
    savedServiceSlugRef.current = snapshot?.service.selectedServerSlug ?? ''
  }, [snapshot?.service.selectedServerSlug])

  useEffect(() => {
    if (!snapshot?.service.authenticated || initialServiceRefreshRef.current) {
      return
    }

    initialServiceRefreshRef.current = true
    let cancelled = false
    setServiceRefreshPhase('catalog')
    void refreshServiceServerCatalog()
      .then((next) => {
        if (cancelled) {
          return null
        }
        startTransition(() => setSnapshot(next))
        setServiceRefreshPhase('status')
        return refreshServiceServers()
      })
      .then((next) => {
        if (!cancelled && next) {
          startTransition(() => setSnapshot(next))
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setErrorMessage(getErrorMessage(error))
        }
      })
      .finally(() => {
        if (!cancelled) {
          setServiceRefreshPhase(null)
        }
      })

    return () => {
      cancelled = true
    }
  }, [snapshot?.service.authenticated])

  useEffect(() => {
    if (renamingThemeId && renameInputRef.current) {
      renameInputRef.current.focus()
      renameInputRef.current.select()
    }
  }, [renamingThemeId])

  useEffect(() => {
    if (newThemeDraft && newNameInputRef.current) {
      newNameInputRef.current.focus()
    }
  }, [newThemeDraft])

  async function handleThemeChange(themeId: string) {
    if (renamingThemeId === themeId) {
      return
    }
    try {
      setBusyAction(`theme:${themeId}`)
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

  function startNewTheme() {
    setNewThemeDraft({ name: '', accent: DEFAULT_NEW_THEME_ACCENT })
  }

  function cancelNewTheme() {
    setNewThemeDraft(null)
  }

  async function handleCreateTheme() {
    if (!newThemeDraft) {
      return
    }
    try {
      setBusyAction('create-theme')
      setErrorMessage(null)
      const next = await createCustomTheme({
        name: newThemeDraft.name.trim(),
        accent: newThemeDraft.accent,
      })
      startTransition(() => setSnapshot(next))
      setNewThemeDraft(null)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  function startRename(theme: CustomThemeSnapshot) {
    setRenamingThemeId(theme.id)
    setRenameDraft(theme.name)
  }

  function cancelRename() {
    setRenamingThemeId(null)
    setRenameDraft('')
  }

  async function commitRename() {
    if (!renamingThemeId) {
      return
    }
    const name = renameDraft.trim()
    if (!name) {
      cancelRename()
      return
    }
    try {
      setBusyAction(`rename:${renamingThemeId}`)
      setErrorMessage(null)
      const next = await renameCustomTheme({ id: renamingThemeId, name })
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
      cancelRename()
    }
  }

  async function handleAccentChange(themeId: string, accent: string) {
    try {
      setBusyAction(`accent:${themeId}`)
      setErrorMessage(null)
      const next = await updateCustomThemeAccent({ id: themeId, accent })
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleDeleteTheme(themeId: string) {
    try {
      setBusyAction(`delete:${themeId}`)
      setErrorMessage(null)
      const next = await deleteCustomTheme({ id: themeId })
      startTransition(() => setSnapshot(next))
      if (renamingThemeId === themeId) {
        cancelRename()
      }
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleWorkspaceOpen(selectedServerSlug?: string) {
    const service = snapshot?.service
    const targetServer =
      service?.servers.find((server) => server.slug === selectedServerSlug) ??
      service?.servers.find((server) => server.slug === service.selectedServerSlug) ??
      service?.servers.find((server) => server.selected) ??
      null

    try {
      setBusyAction('workspace')
      setWorkspaceLaunchActive(true)
      setWorkspaceLaunchTarget(targetServer?.name ?? selectedServerSlug ?? null)
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await openWorkspace(selectedServerSlug)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setWorkspaceLaunchActive(false)
      setWorkspaceLaunchTarget(null)
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleServiceRefresh() {
    try {
      setBusyAction('refresh-service')
      setServiceRefreshPhase('catalog')
      setErrorMessage(null)
      await waitForNextPaint()
      const catalog = await refreshServiceServerCatalog()
      startTransition(() => setSnapshot(catalog))
      setServiceRefreshPhase('status')
      await waitForNextPaint()
      const next = await refreshServiceServers()
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setServiceRefreshPhase(null)
      setBusyAction(null)
    }
  }

  async function handleServiceStop() {
    if (!snapshot) {
      return
    }

    const currentCopy = getCopy(snapshot.language, snapshot.resolvedLanguage)
    const selectedServer =
      snapshot.service.servers.find(
        (server) => server.slug === snapshot.service.selectedServerSlug,
      ) ??
      snapshot.service.servers.find((server) => server.selected) ??
      snapshot.service.servers[0]
    const selectedSlug = selectedServer?.slug ?? snapshot.service.selectedServerSlug

    if (!selectedSlug) {
      setErrorMessage(currentCopy.serviceNotRunning)
      return
    }

    try {
      setBusyAction('stop-service')
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await stopService(selectedSlug)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleServiceStart() {
    if (!snapshot) {
      return
    }

    const currentCopy = getCopy(snapshot.language, snapshot.resolvedLanguage)
    const selectedServer =
      snapshot.service.servers.find(
        (server) => server.slug === snapshot.service.selectedServerSlug,
      ) ??
      snapshot.service.servers.find((server) => server.selected) ??
      snapshot.service.servers[0]
    const selectedSlug = selectedServer?.slug ?? snapshot.service.selectedServerSlug

    if (!selectedSlug) {
      setErrorMessage(currentCopy.selectedServerPlaceholder)
      return
    }

    try {
      setBusyAction('start-service')
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await startService(selectedSlug)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleServiceServerSelect(selectedServerSlug: string) {
    try {
      setBusyAction(`select-service:${selectedServerSlug}`)
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await selectServiceServer(selectedServerSlug)
      startTransition(() => setSnapshot(next))
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
      await waitForNextPaint()

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
        installing: false,
        error: null,
        latest,
      })
    } catch (error) {
      setReleaseState({
        loading: false,
        installing: false,
        error: getErrorMessage(error),
        latest: null,
      })
    }
  }

  async function handleDesktopUpdateInstall() {
    try {
      setReleaseState((current) => ({
        ...current,
        installing: true,
        error: null,
      }))
      await waitForNextPaint()
      await installDesktopUpdate()
    } catch (error) {
      setReleaseState((current) => ({
        ...current,
        installing: false,
        error: getErrorMessage(error),
      }))
    }
  }

  if (!snapshot) {
    const bootCopy = getCopy('system')

    return (
      <main className="loading-shell">
        <SlockBrandMark className="loading-mark" />
        <SpinnerIcon />
        <p className="eyebrow">{bootCopy.appBootingTitle}</p>
        <p className="loading-detail">{bootCopy.appBootingDetail}</p>
      </main>
    )
  }

  const activeTheme =
    snapshot.themes.find((theme) => theme.id === snapshot.colorScheme) ??
    snapshot.themes[0]
  const copy = getCopy(snapshot.language, snapshot.resolvedLanguage)
  const shellStyle = buildShellStyle(activeTheme)
  const stackButtonLabel = snapshot.workspaceOpen ? copy.focusSlock : copy.openSlock
  const selectedServiceServer =
    snapshot.service.servers.find(
      (server) => server.slug === snapshot.service.selectedServerSlug,
    ) ??
    snapshot.service.servers.find((server) => server.selected) ??
    snapshot.service.servers[0] ??
    null
  const savedServiceSlug = snapshot.service.selectedServerSlug.trim()
  const selectedServiceSlug = selectedServiceServer?.slug ?? savedServiceSlug
  const canOpenWorkspace =
    snapshot.service.authenticated && Boolean(selectedServiceSlug)
  const normalizedServerQuery = serverQuery.trim().toLowerCase()
  const filteredServiceServers = normalizedServerQuery
    ? snapshot.service.servers.filter((server) => {
        const machineName = server.machineName ?? ''
        return `${server.name} ${server.slug} ${machineName}`
          .toLowerCase()
          .includes(normalizedServerQuery)
      })
    : snapshot.service.servers
  const serviceStatusLabel = getServiceStatusLabel(
    snapshot.service,
    selectedServiceServer,
    copy,
  )
  const serviceRefreshing = Boolean(serviceRefreshPhase) || busyAction === 'refresh-service'
  const serviceBusyMessage = getServiceBusyMessage(
    busyAction,
    serviceRefreshPhase,
    copy,
  )
  const serviceActionBusy =
    busyAction === 'start-service' ||
    busyAction === 'stop-service' ||
    busyAction === 'workspace' ||
    busyAction === 'refresh-service' ||
    workspaceLaunchActive ||
    Boolean(workspaceLaunchTarget) ||
    Boolean(busyAction?.startsWith('select-service:'))
  const workspaceLaunching =
    busyAction === 'workspace' ||
    workspaceLaunchActive ||
    Boolean(workspaceLaunchTarget) ||
    snapshot.workspaceOpen
  const activeIsOriginal = snapshot.colorScheme === 'original' || !snapshot.colorScheme
  const releaseIsCurrent =
    Boolean(releaseState.latest) && !releaseState.latest?.updateAvailable
  const releaseUpdateAvailable = Boolean(releaseState.latest?.updateAvailable)
  const releaseStatusLabel = releaseState.latest
    ? releaseState.latest.updateAvailable
      ? copy.updateAvailable
      : copy.current
    : copy.notChecked

  return (
    <main
      className="studio-shell"
      data-mode={activeTheme.mode}
      style={shellStyle}
      aria-busy={workspaceLaunching}
    >
      <section className="launch-board" aria-label={copy.openSlock}>
        {workspaceLaunching ? (
          <section className="workspace-loading-overlay" aria-live="polite">
            <div className="workspace-loading-panel">
              <div className="workspace-loading-mark">
                <SlockBrandMark />
                <span className="workspace-loading-ring" aria-hidden="true" />
              </div>
              <div className="workspace-loading-copy">
                <p className="eyebrow">{copy.launchingTitle}</p>
                <p>{workspaceLaunchTarget ?? copy.launchingDetail}</p>
              </div>
            </div>
          </section>
        ) : null}

        {errorMessage ? (
          <section className="error-banner" role="alert">
            <strong>{copy.desktopStateError}</strong>
            <p>{errorMessage}</p>
          </section>
        ) : null}

        <header className="launch-bar">
          <div className="launch-bar-brand">
            <SlockBrandMark className="launch-bar-mark" />
            <span className="launch-bar-wordmark">slock.ai</span>
          </div>

          <div className="launch-bar-controls">
            <span className={`status-pill${snapshot.workspaceOpen ? ' live' : ''}`}>
              {snapshot.workspaceOpen ? copy.workspaceActive : copy.workspaceParked}
            </span>
            <span className={`status-pill${snapshot.service.running ? ' live' : ''}`}>
              {serviceStatusLabel}
            </span>
            <div className="icon-segment compact-icons" role="radiogroup" aria-label={copy.mode}>
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
                    <OptionIcon type={mode.icon} />
                    <span className="sr-only">{copy[mode.labelKey]}</span>
                  </button>
                )
              })}
            </div>
            <div className="icon-segment compact-icons" role="radiogroup" aria-label={copy.language}>
              {LANGUAGE_OPTIONS.map((language) => {
                const selected = language.id === snapshot.language
                return (
                  <button
                    key={language.id}
                    className={`icon-option${selected ? ' selected' : ''}`}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    title={copy[language.labelKey]}
                    onClick={() => handleLanguageChange(language.id)}
                    disabled={busyAction === `language:${language.id}`}
                  >
                    <OptionIcon type={language.icon} />
                    <span className="sr-only">{copy[language.labelKey]}</span>
                  </button>
                )
              })}
            </div>
          </div>
        </header>

        <section className="launch-deck">
          <section className="control-card service-card" aria-label={copy.service}>
            <div className="control-card-head">
              <p className="eyebrow">{copy.service}</p>
              <div className="service-launch-head-actions">
                <span className="server-count-pill">
                  {normalizedServerQuery
                    ? `${filteredServiceServers.length}/${snapshot.service.servers.length}`
                    : `${snapshot.service.servers.length}`}
                </span>
                <button
                  className="icon-action-button positive"
                  onClick={handleServiceStart}
                  disabled={
                    serviceActionBusy ||
                    !snapshot.service.authenticated ||
                    !selectedServiceServer
                  }
                  aria-label={copy.startService}
                  title={copy.startService}
                >
                  <ServiceActionIcon type="start" busy={busyAction === 'start-service'} />
                </button>
                <button
                  className="icon-action-button danger"
                  onClick={handleServiceStop}
                  disabled={
                    serviceActionBusy ||
                    !snapshot.service.authenticated ||
                    !selectedServiceServer
                  }
                  aria-label={copy.closeServer}
                  title={copy.closeServer}
                >
                  <ServiceActionIcon type="stop" busy={busyAction === 'stop-service'} />
                </button>
                <button
                  className="icon-action-button"
                  onClick={handleServiceRefresh}
                  disabled={serviceRefreshing}
                  aria-label={copy.refreshServers}
                  title={copy.refreshServers}
                >
                  <ServiceActionIcon type="refresh" busy={serviceRefreshing} />
                </button>
              </div>
            </div>

            {snapshot.service.syncError ? (
              <p className="inline-note error">{snapshot.service.syncError}</p>
            ) : snapshot.service.lastError ? (
              <p className="inline-note error">{snapshot.service.lastError}</p>
            ) : !snapshot.service.authenticated ? (
              <p className="inline-note">{copy.serviceSignInHint}</p>
            ) : snapshot.service.servers.length === 0 && !serviceRefreshing ? (
              <p className="inline-note">{copy.noServers}</p>
            ) : null}

            {snapshot.service.servers.length > 0 ? (
              <label className="server-search">
                <ServerSearchIcon />
                <span className="sr-only">{copy.serverSearch}</span>
                <input
                  value={serverQuery}
                  onChange={(event) => setServerQuery(event.target.value)}
                  placeholder={copy.serverSearch}
                  aria-label={copy.serverSearch}
                />
              </label>
            ) : null}

            <div className="service-server-list" role="list" aria-label={copy.selectedServer}>
              {serviceBusyMessage ? (
                <div className="service-loading-row" role="status" aria-live="polite">
                  <SpinnerIcon />
                  <span>{serviceBusyMessage}</span>
                </div>
              ) : null}
              {filteredServiceServers.map((server) => {
                const selected = server.slug === selectedServiceSlug
                const selecting = busyAction === `select-service:${server.slug}`
                const running =
                  snapshot.service.running &&
                  server.slug === snapshot.service.activeServerSlug
                const serverStatusLabel = getServiceServerStatusLabel(
                  server,
                  snapshot.service,
                  copy,
                  selectedServiceSlug,
                )

                return (
                  <button
                    key={server.id}
                    className={`service-server-row${selected ? ' selected' : ''}${running ? ' running' : ''}`}
                    type="button"
                    aria-pressed={selected}
                    disabled={
                      busyAction?.startsWith('select-service:') ||
                      busyAction === 'start-service' ||
                      busyAction === 'workspace' ||
                      busyAction === 'stop-service' ||
                      busyAction === 'refresh-service'
                    }
                    onClick={() => handleServiceServerSelect(server.slug)}
                  >
                    <span className="service-server-copy">
                      <span className="service-server-name-line">
                        <span className="service-server-name">{server.name}</span>
                        <span className="service-server-slug">{server.slug}</span>
                      </span>
                    </span>
                    <span className={`status-chip${running ? ' live' : ''}`}>
                      {selecting ? copy.saving : serverStatusLabel}
                    </span>
                  </button>
                )
              })}
              {snapshot.service.servers.length > 0 && filteredServiceServers.length === 0 ? (
                <p className="inline-note">{copy.noMatchingServers}</p>
              ) : null}
            </div>
          </section>

          <section className="control-card theme-card" aria-label={copy.appearance}>
            <div className="control-card-head">
              <p className="eyebrow">{copy.appearance}</p>
              <button
                className="icon-action-button positive theme-add-button"
                type="button"
                onClick={startNewTheme}
                disabled={Boolean(newThemeDraft) || busyAction === 'create-theme'}
                aria-label={copy.themeNewLabel}
                title={copy.themeNewLabel}
              >
                <PlusIcon />
              </button>
            </div>

            <ul className="theme-rail" role="radiogroup" aria-label={copy.themeColor}>
              <li>
                <ThemeRow
                  themeId="original"
                  swatch={ORIGINAL_SWATCH}
                  name={copy.themeOriginalName}
                  summary={copy.themeOriginalSummary}
                  selected={activeIsOriginal}
                  busy={busyAction === 'theme:original'}
                  locked
                  lockedLabel={copy.themeBuiltIn}
                  onSelect={() => handleThemeChange('original')}
                />
              </li>
              {snapshot.customThemes.map((theme) => {
                const selected = theme.id === snapshot.colorScheme
                const isRenaming = theme.id === renamingThemeId
                const summary = `${theme.accent.toUpperCase()}`
                return (
                  <li key={theme.id}>
                    <ThemeRow
                      themeId={theme.id}
                      swatch={theme.accent}
                      name={theme.name}
                      summary={summary}
                      selected={selected}
                      busy={busyAction === `theme:${theme.id}`}
                      renaming={isRenaming}
                      renameDraft={renameDraft}
                      onRenameDraftChange={setRenameDraft}
                      renameInputRef={renameInputRef}
                      onCommitRename={() => void commitRename()}
                      onCancelRename={cancelRename}
                      onSelect={() => handleThemeChange(theme.id)}
                      onStartRename={() => startRename(theme)}
                      onAccentChange={(value) => void handleAccentChange(theme.id, value)}
                      onDelete={() => void handleDeleteTheme(theme.id)}
                      deleting={busyAction === `delete:${theme.id}`}
                      renameLabel={copy.themeRename}
                      deleteLabel={copy.themeDelete}
                      accentLabel={copy.customThemeAccentAria}
                    />
                  </li>
                )
              })}
              {snapshot.customThemes.length === 0 && !newThemeDraft ? (
                <li className="theme-empty-row">
                  <p className="inline-note">{copy.themeEmptyHint}</p>
                </li>
              ) : null}
              {newThemeDraft ? (
                <li className="theme-draft-row">
                  <label
                    className="accent-wheel"
                    style={{ '--custom-accent': newThemeDraft.accent } as CSSProperties}
                  >
                    <span className="sr-only">{copy.customThemeAccentAria}</span>
                    <input
                      type="color"
                      value={newThemeDraft.accent}
                      onChange={(event) =>
                        setNewThemeDraft((current) =>
                          current ? { ...current, accent: event.target.value } : current,
                        )
                      }
                      aria-label={copy.customThemeAccentAria}
                    />
                  </label>
                  <input
                    ref={newNameInputRef}
                    className="theme-name-input"
                    value={newThemeDraft.name}
                    onChange={(event) =>
                      setNewThemeDraft((current) =>
                        current ? { ...current, name: event.target.value } : current,
                      )
                    }
                    placeholder={copy.customThemeNamePlaceholder}
                    aria-label={copy.themeNewTitle}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') {
                        event.preventDefault()
                        void handleCreateTheme()
                      } else if (event.key === 'Escape') {
                        event.preventDefault()
                        cancelNewTheme()
                      }
                    }}
                  />
                  <div className="theme-row-actions">
                    <button
                      className="theme-button accent-save-button"
                      type="button"
                      onClick={handleCreateTheme}
                      disabled={busyAction === 'create-theme'}
                    >
                      {busyAction === 'create-theme' ? copy.creatingTheme : copy.themeCreate}
                    </button>
                    <button
                      className="theme-button muted-button"
                      type="button"
                      onClick={cancelNewTheme}
                    >
                      {copy.themeRenameCancel}
                    </button>
                  </div>
                </li>
              ) : null}
            </ul>
          </section>

          <section className="control-card update-card">
            <div className="control-card-head version-card-head">
              <div>
                <p className="eyebrow">{copy.releaseCheck}</p>
                <p className="version-current">
                  {copy.currentVersion} {snapshot.updates.currentVersion}
                </p>
              </div>
              <span className={`status-chip ${releaseUpdateAvailable ? 'warm' : ''}`}>
                {releaseStatusLabel}
              </span>
            </div>

            <div className="control-body">
              {releaseState.error ? (
                <p className="inline-note error">{releaseState.error}</p>
              ) : releaseState.latest?.updateAvailable ? (
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
                </div>
              ) : null}

              <div className="button-row">
                <button
                  className={`theme-button${releaseIsCurrent ? ' static-disabled' : ''}`}
                  onClick={handleReleaseCheck}
                  disabled={
                    releaseState.loading ||
                    releaseState.installing ||
                    releaseIsCurrent
                  }
                >
                  {releaseState.loading
                    ? copy.checkingRelease
                    : releaseIsCurrent
                      ? copy.current
                      : copy.checkGitHubRelease}
                </button>
                {releaseUpdateAvailable ? (
                  <button
                    className="theme-button accent-save-button"
                    onClick={handleDesktopUpdateInstall}
                    disabled={releaseState.installing || releaseState.loading}
                  >
                    {releaseState.installing
                      ? copy.installingDesktopUpdate
                      : copy.installDesktopUpdate}
                  </button>
                ) : null}
              </div>
            </div>
          </section>
        </section>

        <div className="launch-dock">
          <button
            className="launch-button launch-button-bottom"
            onClick={() => handleWorkspaceOpen(selectedServiceSlug || undefined)}
            disabled={
              serviceActionBusy ||
              !canOpenWorkspace
            }
          >
            {busyAction === 'workspace' ? copy.launching : stackButtonLabel}
          </button>
        </div>
      </section>
    </main>
  )
}

interface ThemeRowProps {
  themeId: string
  swatch: string
  name: string
  summary: string
  selected: boolean
  busy: boolean
  locked?: boolean
  lockedLabel?: string
  renaming?: boolean
  renameDraft?: string
  onRenameDraftChange?: (value: string) => void
  renameInputRef?: React.RefObject<HTMLInputElement | null>
  onCommitRename?: () => void
  onCancelRename?: () => void
  onSelect: () => void
  onStartRename?: () => void
  onAccentChange?: (value: string) => void
  onDelete?: () => void
  deleting?: boolean
  renameLabel?: string
  deleteLabel?: string
  accentLabel?: string
}

function ThemeRow(props: ThemeRowProps) {
  const {
    swatch,
    name,
    summary,
    selected,
    busy,
    locked,
    lockedLabel,
    renaming,
    renameDraft,
    onRenameDraftChange,
    renameInputRef,
    onCommitRename,
    onCancelRename,
    onSelect,
    onStartRename,
    onAccentChange,
    onDelete,
    deleting,
    renameLabel,
    deleteLabel,
    accentLabel,
  } = props

  return (
    <div
      className={`theme-row${selected ? ' selected' : ''}${locked ? ' locked' : ''}`}
      role="radio"
      aria-checked={selected}
      tabIndex={0}
      onKeyDown={(event) => {
        if ((event.key === 'Enter' || event.key === ' ') && !renaming) {
          event.preventDefault()
          onSelect()
        }
      }}
      onClick={(event) => {
        if (renaming) {
          return
        }
        const target = event.target as HTMLElement
        if (target.closest('button, input, label')) {
          return
        }
        onSelect()
      }}
    >
      {locked ? (
        <span
          className="theme-row-swatch locked"
          style={{ background: swatch }}
          aria-hidden="true"
        />
      ) : (
        <label
          className="theme-row-swatch interactive"
          style={{ background: swatch }}
          title={accentLabel}
        >
          <span className="sr-only">{accentLabel}</span>
          <input
            type="color"
            value={swatch}
            onChange={(event) => onAccentChange?.(event.target.value)}
            aria-label={accentLabel}
          />
        </label>
      )}

      <div className="theme-row-copy">
        {renaming ? (
          <input
            ref={renameInputRef}
            className="theme-name-input"
            value={renameDraft ?? ''}
            onChange={(event) => onRenameDraftChange?.(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                onCommitRename?.()
              } else if (event.key === 'Escape') {
                event.preventDefault()
                onCancelRename?.()
              }
            }}
            onBlur={() => onCommitRename?.()}
            aria-label={renameLabel}
          />
        ) : (
          <>
            <span className="theme-row-name">{name}</span>
            <span className="theme-row-summary">{locked ? lockedLabel : summary}</span>
          </>
        )}
      </div>

      {!locked ? (
        <div className="theme-row-actions">
          <button
            className="icon-action-button compact"
            type="button"
            onClick={onStartRename}
            disabled={busy || renaming}
            aria-label={renameLabel}
            title={renameLabel}
          >
            <PencilIcon />
          </button>
          <button
            className="icon-action-button danger compact"
            type="button"
            onClick={onDelete}
            disabled={deleting}
            aria-label={deleteLabel}
            title={deleteLabel}
          >
            {deleting ? <SpinnerIcon /> : <TrashIcon />}
          </button>
        </div>
      ) : null}
    </div>
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

type ServiceActionIconType = 'start' | 'stop' | 'refresh'
type OptionIconType =
  | (typeof THEME_MODES)[number]['icon']
  | (typeof LANGUAGE_OPTIONS)[number]['icon']

function OptionIcon({ type }: { type: OptionIconType }) {
  if (type === 'latin' || type === 'han') {
    return (
      <svg
        className="option-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
      >
        <text
          x="12"
          y="16"
          textAnchor="middle"
          fill="currentColor"
          fontSize="13"
          fontFamily="Avenir Next, SF Pro Display, PingFang SC, sans-serif"
          fontWeight="800"
        >
          {type === 'latin' ? 'A' : '文'}
        </text>
      </svg>
    )
  }

  if (type === 'sun') {
    return (
      <svg
        className="option-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2" />
        <path d="M12 20v2" />
        <path d="m4.9 4.9 1.4 1.4" />
        <path d="m17.7 17.7 1.4 1.4" />
        <path d="M2 12h2" />
        <path d="M20 12h2" />
        <path d="m4.9 19.1 1.4-1.4" />
        <path d="m17.7 6.3 1.4-1.4" />
      </svg>
    )
  }

  if (type === 'moon') {
    return (
      <svg
        className="option-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M20.4 14.5A7.7 7.7 0 0 1 9.5 3.6 8.7 8.7 0 1 0 20.4 14.5Z" />
      </svg>
    )
  }

  if (type === 'display') {
    return (
      <svg
        className="option-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect width="16" height="11" x="4" y="5" rx="2" />
        <path d="M12 16v3" />
        <path d="M8 19h8" />
      </svg>
    )
  }

  return (
    <svg
      className="option-icon"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18" />
      <path d="M12 3a14 14 0 0 1 0 18" />
      <path d="M12 3a14 14 0 0 0 0 18" />
    </svg>
  )
}

function ServerSearchIcon() {
  return (
    <svg
      className="server-search-icon"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.2-3.2" />
    </svg>
  )
}

function PlusIcon() {
  return (
    <svg
      className="service-action-icon"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M12 5v14" />
      <path d="M5 12h14" />
    </svg>
  )
}

function PencilIcon() {
  return (
    <svg
      className="service-action-icon"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.121 2.121 0 1 1 3 3L7 19l-4 1 1-4Z" />
    </svg>
  )
}

function TrashIcon() {
  return (
    <svg
      className="service-action-icon"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M3 6h18" />
      <path d="M8 6V4h8v2" />
      <path d="M10 11v6" />
      <path d="M14 11v6" />
      <path d="M6 6l1 15h10l1-15" />
    </svg>
  )
}

function SpinnerIcon() {
  return (
    <svg
      className="service-action-icon spinning"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M21 12a9 9 0 0 1-9 9" />
      <path d="M3 12a9 9 0 0 1 9-9" />
    </svg>
  )
}

function ServiceActionIcon({
  type,
  busy = false,
}: {
  type: ServiceActionIconType
  busy?: boolean
}) {
  if (busy) {
    return <SpinnerIcon />
  }

  if (type === 'start') {
    return (
      <svg
        className="service-action-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <polygon points="9 7 17 12 9 17 9 7" />
      </svg>
    )
  }

  if (type === 'stop') {
    return (
      <svg
        className="service-action-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M12 3v9" />
        <path d="M18.4 6.6a8 8 0 1 1-12.8 0" />
      </svg>
    )
  }

  return (
    <svg
      className="service-action-icon"
      aria-hidden="true"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M21 2v6h-6" />
      <path d="M21 8A9 9 0 1 0 12 21a9 9 0 0 0 8.2-5.3" />
    </svg>
  )
}

function SlockBrandMark({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      width="26"
      height="25"
      viewBox="0 0 48 46"
      fill="none"
      aria-hidden="true"
    >
      <path
        fill="currentColor"
        d="M25.946 44.938c-.664.845-2.021.375-2.021-.698V33.937a2.26 2.26 0 0 0-2.262-2.262H10.287c-.92 0-1.456-1.04-.92-1.788l7.48-10.471c1.07-1.497 0-3.578-1.842-3.578H1.237c-.92 0-1.456-1.04-.92-1.788L10.013.474c.214-.297.556-.474.92-.474h28.894c.92 0 1.456 1.04.92 1.788l-7.48 10.471c-1.07 1.498 0 3.579 1.842 3.579h11.377c.943 0 1.473 1.088.89 1.83L25.947 44.94z"
      />
    </svg>
  )
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

function getServiceStatusLabel(
  service: BootstrapPayload['service'],
  selectedServer: BootstrapPayload['service']['servers'][number] | null,
  copy: UiCopy,
) {
  const selectedSlug = selectedServer?.slug ?? service.selectedServerSlug
  const selectedRunning =
    service.running &&
    Boolean(selectedSlug) &&
    (!service.activeServerSlug || service.activeServerSlug === selectedSlug)

  if (selectedRunning) {
    return copy.serviceRunning
  }

  if (!service.authenticated) {
    return copy.serviceSignInRequired
  }

  if (!selectedServer) {
    return copy.notConfigured
  }

  return service.configured ? copy.serviceIdle : copy.serviceNotLinked
}

function getMachineStatusLabel(
  status: string,
  copy: UiCopy,
) {
  switch (status.trim().toLowerCase()) {
    case 'online':
    case 'running':
    case 'healthy':
    case 'idle':
    case 'ready':
      return copy.serviceRunning
    case 'offline':
    case 'stopped':
      return copy.serviceOffline
    case 'not linked':
      return copy.serviceNotLinked
    default:
      return status || copy.notConfigured
  }
}

function getServiceServerStatusLabel(
  server: BootstrapPayload['service']['servers'][number],
  service: BootstrapPayload['service'],
  copy: UiCopy,
  activeServerSlug = service.selectedServerSlug,
) {
  const runningServerSlug = service.activeServerSlug || activeServerSlug
  if (service.running && server.slug === runningServerSlug) {
    return copy.serviceRunning
  }

  if (server.slug === activeServerSlug || server.selected) {
    return service.configured ? copy.serviceIdle : copy.serviceNotLinked
  }

  return getMachineStatusLabel(server.machineStatus, copy)
}

function getServiceBusyMessage(
  busyAction: string | null,
  serviceRefreshPhase: ServiceRefreshPhase,
  copy: UiCopy,
) {
  if (busyAction === 'start-service') {
    return copy.startingSelectedServer
  }

  if (busyAction === 'stop-service') {
    return copy.closingSelectedServer
  }

  if (busyAction?.startsWith('select-service:')) {
    return copy.savingSelectedServer
  }

  if (serviceRefreshPhase === 'catalog') {
    return copy.loadingServerCatalog
  }

  if (serviceRefreshPhase === 'status' || busyAction === 'refresh-service') {
    return copy.syncingServerStatus
  }

  return null
}

function getErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return typeof error === 'string' ? error : 'Unknown desktop error'
}

function waitForNextPaint() {
  return new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
  })
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
    published_at?: string
    body?: string
    prerelease?: boolean
  }

  const tagName = release.tag_name ?? 'unknown'
  return {
    tagName,
    name: release.name ?? '',
    publishedAt: release.published_at ?? '',
    body: release.body ?? '',
    prerelease: Boolean(release.prerelease),
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
