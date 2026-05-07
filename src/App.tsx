import {
  type ChangeEvent as ReactChangeEvent,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
  startTransition,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { listen } from '@tauri-apps/api/event'
import './App.css'
import './Settings.css'
import {
  activateAccount,
  type AgentActivityEntry,
  type BootstrapPayload,
  type CustomThemeSnapshot,
  type DashboardAgent,
  type DashboardData,
  type DesktopUpdateCheck,
  type ServiceAccountSnapshot,
  type ServiceLogSnapshot,
  type ThemeDefinition,
  type ThemeStyleConfig,
  type ThemeStyleDefinition,
  checkDesktopUpdate,
  createCustomTheme,
  deleteCustomTheme,
  fetchAgentActivity,
  fetchDashboard,
  installDesktopUpdate,
  forgetAccount,
  importThemeStyle,
  loadBootstrap,
  openLogin,
  openServiceLog,
  openWorkspace,
  refreshServiceServerCatalog,
  refreshServiceServerStatus,
  renameCustomTheme,
  selectServiceServer,
  startAgent,
  startService,
  stopAgent,
  stopService,
  switchAccount,
  updateCustomThemeAccent,
  updateLanguage,
  updateTheme,
  updateThemeMode,
  updateThemeStyle,
} from './lib/desktop'

interface ReleaseState {
  loading: boolean
  installing: boolean
  error: string | null
  latest: DesktopUpdateCheck | null
}

const INITIAL_RELEASE_STATE: ReleaseState = {
  loading: false,
  installing: false,
  error: null,
  latest: null,
}

const MESSAGE_REMINDER_TOAST_MS = 7000

interface MessageReminderToast {
  id: string
  channelId: string
  serverId: string
  serverSlug: string
  serverName: string
  senderName: string
  senderId?: string
  senderType?: string
  contentPreview: string
}

const DEFAULT_NEW_THEME_ACCENT = '#10a37f'
const THEME_ACCENT_PRESETS = [
  '#ff3b30',
  '#ff9500',
  '#ffcc00',
  '#34c759',
  '#32ade6',
  '#007aff',
  '#af52de',
] as const

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

const AUTH_POLL_INTERVAL_MS = 1000
const AUTH_POLL_MAX_ATTEMPTS = 180
const AUTH_INTERRUPTED_HINT_MS = 1800
const DEFAULT_SERVICE_LOG_RANGE_MS = 30 * 60 * 1000
const SERVICE_LOG_QUICK_RANGES = [
  { key: 'serverLogQuick30s', durationMs: 30 * 1000 },
  { key: 'serverLogQuick1m', durationMs: 60 * 1000 },
  { key: 'serverLogQuick5m', durationMs: 5 * 60 * 1000 },
  { key: 'serverLogQuick30m', durationMs: DEFAULT_SERVICE_LOG_RANGE_MS },
  { key: 'serverLogQuick1h', durationMs: 60 * 60 * 1000 },
] as const

const COPY = {
  'en-US': {
    workspaceActive: 'Workspace active',
    workspaceParked: 'Workspace parked',
    appearance: 'Theme',
    service: 'Server',
    updates: 'Updates',
    mode: 'Mode',
    themeColor: 'Accent color',
    themeStyle: 'Style',
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
    messageReminderTitle: 'New message',
    messageReminderOpen: 'Open',
    messageReminderDismiss: 'Dismiss',
    launching: 'Launching…',
    launchingTitle: 'Opening Slock',
    launchingDetail: 'Preparing workspace',
    browserLoginPending: 'Complete sign-in in the Slock window',
    loginInterrupted: 'Sign-in interrupted',
    signedIn: 'Signed in',
    signIn: 'Sign in',
    switchAccount: 'Switch account',
    addAccount: 'Add account',
    forgetAccount: 'Remove account',
    currentAccount: 'Current',
    accountEmailUnavailable: 'Signed in',
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
  openServerLog: 'View server logs',
    serverLogTitle: 'Server logs',
    serverLogSearch: 'Search logs',
    serverLogSearching: 'Searching…',
    serverLogFrom: 'From',
    serverLogTo: 'To',
    serverLogRange: 'Range',
    serverLogCustomRange: 'Custom',
    serverLogRangeApply: 'Load range',
    serverLogQuick30s: '30s',
    serverLogQuick1m: '1m',
    serverLogQuick5m: '5m',
    serverLogQuick30m: '30m',
    serverLogQuick1h: '1h',
    serverLogLoading: 'Loading logs…',
    serverLogEmpty: 'Log is empty.',
    serverLogPath: 'Log file',
    serverLogTruncated: 'Showing recent log tail',
    serverLogPreviousMatch: 'Previous match',
    serverLogNextMatch: 'Next match',
    serverLogNoMatches: 'No matches',
    serverLogLines: 'lines',
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
    themeDefaultColorName: 'Default accent',
    themeDefaultColorSummary: 'Slock green.',
    themeStyleOriginalName: 'Original style',
    themeStyleOriginalSummary: 'Current web UI without desktop overrides.',
    themeStyleDefaultName: 'Default style',
    themeStyleDefaultSummary: 'Desktop refined style.',
    themeImportStyle: 'Import style',
    themeExportStyle: 'Export style',
    themeImportInvalid: 'Invalid style file.',
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
    appBootingTitle: 'slock-desktop',
    loginTimeout: 'Sign-in timed out',
    close: 'Close',
    dashboardChannels: 'Channels',
    dashboardUnread: 'Unread',
    dashboardTasks: 'Tasks',
    dashboardAgents: 'Agents',
    dashboardTaskStatus: 'Task Status',
    dashboardAgentStatus: 'Agents',
    dashboardActiveChannels: 'Active Channels',
    dashboardLabel: 'Server Dashboard',
    taskStatusTodo: 'Todo',
    taskStatusInProgress: 'In Progress',
    taskStatusInReview: 'Review',
    taskStatusDone: 'Done',
    dashboardPartialError: 'Some data failed to load',
    agentNoDescription: 'No description',
    agentActivity: 'Recent Activity',
    agentNoActivity: 'No recent activity',
    agentStop: 'Stop',
    agentStart: 'Start',
    agentRestart: 'Restart',
    agentStopping: 'Stopping…',
    agentStarting: 'Starting…',
  },
  'zh-CN': {
    workspaceActive: '工作区已打开',
    workspaceParked: '',
    appearance: '主题',
    service: '服务',
    updates: '更新',
    mode: '模式',
    themeColor: '主题色',
    themeStyle: '样式',
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
    messageReminderTitle: '新消息',
    messageReminderOpen: '打开',
    messageReminderDismiss: '关闭',
    launching: '启动中…',
    launchingTitle: '正在进入 Slock',
    launchingDetail: '正在准备工作区',
    browserLoginPending: '请在 Slock 登录窗口完成登录',
    loginInterrupted: '已中断登录',
    signedIn: '已登录',
    signIn: '登录',
    switchAccount: '切换账号',
    addAccount: '添加账号',
    forgetAccount: '移除账号',
    currentAccount: '当前',
    accountEmailUnavailable: '已登录',
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
  openServerLog: '查看 server 日志',
    serverLogTitle: 'Server 日志',
    serverLogSearch: '搜索日志',
    serverLogSearching: '搜索中…',
    serverLogFrom: '开始',
    serverLogTo: '结束',
    serverLogRange: '范围',
    serverLogCustomRange: '自定义',
    serverLogRangeApply: '加载时间范围',
    serverLogQuick30s: '30秒',
    serverLogQuick1m: '1分钟',
    serverLogQuick5m: '5分钟',
    serverLogQuick30m: '30分钟',
    serverLogQuick1h: '1小时',
    serverLogLoading: '正在读取日志…',
    serverLogEmpty: '日志为空。',
    serverLogPath: '日志文件',
    serverLogTruncated: '正在显示最近的日志尾部',
    serverLogPreviousMatch: '上一条匹配',
    serverLogNextMatch: '下一条匹配',
    serverLogNoMatches: '没有匹配项',
    serverLogLines: '行',
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
    themeDefaultColorName: '默认主题色',
    themeDefaultColorSummary: 'Slock 绿色。',
    themeStyleOriginalName: '原样式',
    themeStyleOriginalSummary: '保留当前 Web UI 原始样式。',
    themeStyleDefaultName: '默认样式',
    themeStyleDefaultSummary: 'Desktop 整理后的样式。',
    themeImportStyle: '导入样式',
    themeExportStyle: '导出样式',
    themeImportInvalid: '样式文件无效。',
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
    appBootingTitle: 'slock-desktop',
    loginTimeout: '登录超时',
    close: '关闭',
    dashboardChannels: '频道',
    dashboardUnread: '未读',
    dashboardTasks: '任务',
    dashboardAgents: 'Agents',
    dashboardTaskStatus: '任务状态',
    dashboardAgentStatus: 'Agents',
    dashboardActiveChannels: '活跃频道',
    dashboardLabel: 'Server 概览',
    taskStatusTodo: '待办',
    taskStatusInProgress: '进行中',
    taskStatusInReview: '审核中',
    taskStatusDone: '已完成',
    dashboardPartialError: '部分数据加载失败',
    agentNoDescription: '无描述',
    agentActivity: '最近活动',
    agentNoActivity: '暂无活动记录',
    agentStop: '停止',
    agentStart: '启动',
    agentRestart: '重启',
    agentStopping: '停止中…',
    agentStarting: '启动中…',
  },
} as const

type UiCopy = (typeof COPY)[keyof typeof COPY]
type ServiceRefreshPhase = 'catalog' | 'status' | null

interface NewThemeDraft {
  name: string
  accent: string
  hexInput: string
  rgbInput: {
    r: string
    g: string
    b: string
  }
}

type RgbChannel = keyof NewThemeDraft['rgbInput']

interface ServiceLogViewerState {
  loading: boolean
  snapshot: ServiceLogSnapshot | null
  serverSlug: string
  serverName: string
  query: string
  rangeStart: string
  rangeEnd: string
  rangePresetMs: number | null
  activeMatchIndex: number
  error: string | null
}

interface ServiceLogSearchState {
  query: string
  activeMatchIndex: number
  count: number
  activeStart: number
  activeEnd: number
  searching: boolean
}

const EMPTY_SERVICE_LOG_SEARCH: ServiceLogSearchState = {
  query: '',
  activeMatchIndex: 0,
  count: 0,
  activeStart: -1,
  activeEnd: -1,
  searching: false,
}

function App() {
  const [snapshot, setSnapshot] = useState<BootstrapPayload | null>(null)
  const [busyAction, setBusyAction] = useState<string | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [releaseState, setReleaseState] = useState<ReleaseState>(INITIAL_RELEASE_STATE)
  const [serverQuery, setServerQuery] = useState('')
  const [serviceLogViewer, setServiceLogViewer] =
    useState<ServiceLogViewerState | null>(null)
  const [serviceLogSearch, setServiceLogSearch] = useState<ServiceLogSearchState>(
    EMPTY_SERVICE_LOG_SEARCH,
  )
  const [workspaceLaunchActive, setWorkspaceLaunchActive] = useState(false)
  const [workspaceLaunchTarget, setWorkspaceLaunchTarget] = useState<string | null>(null)
  const [browserLoginPending, setBrowserLoginPending] = useState(false)
  const [serviceRefreshPhase, setServiceRefreshPhase] = useState<ServiceRefreshPhase>(null)
  const [renamingThemeId, setRenamingThemeId] = useState<string | null>(null)
  const [renameDraft, setRenameDraft] = useState('')
  const [newThemeDraft, setNewThemeDraft] = useState<NewThemeDraft | null>(null)
  const [newThemeWheelOpen, setNewThemeWheelOpen] = useState(false)
  const [accountMenuOpen, setAccountMenuOpen] = useState(false)
  const [serverPanelOpen, setServerPanelOpen] = useState(false)
  const [themePanelOpen, setThemePanelOpen] = useState(false)
  const [stylePanelOpen, setStylePanelOpen] = useState(false)
  const accountMenuRef = useRef<HTMLDivElement | null>(null)
  const serverPanelRef = useRef<HTMLDivElement | null>(null)
  const themePanelRef = useRef<HTMLDivElement | null>(null)
  const stylePanelRef = useRef<HTMLDivElement | null>(null)
  const themeDraftRef = useRef<HTMLDivElement | null>(null)
  const renameInputRef = useRef<HTMLInputElement | null>(null)
  const newNameInputRef = useRef<HTMLInputElement | null>(null)
  const styleImportInputRef = useRef<HTMLInputElement | null>(null)
  const serviceLogSearchRef = useRef<HTMLInputElement | null>(null)
  const serviceLogContentRef = useRef<HTMLPreElement | null>(null)
  const [dashboardData, setDashboardData] = useState<DashboardData | null>(null)
  const [dashboardLoading, setDashboardLoading] = useState(false)
  const [dashboardError, setDashboardError] = useState<string | null>(null)
  const [messageReminder, setMessageReminder] = useState<MessageReminderToast | null>(null)
  const messageReminderTimerRef = useRef<number | null>(null)
  const [agentCardTarget, setAgentCardTarget] = useState<DashboardAgent | null>(null)
  const [agentCardActivity, setAgentCardActivity] = useState<AgentActivityEntry[]>([])
  const [agentCardLoading, setAgentCardLoading] = useState(false)
  const [agentCardAction, setAgentCardAction] = useState<string | null>(null)
  const agentCardRef = useRef<HTMLDivElement | null>(null)
  const initialServiceRefreshRef = useRef(false)
  const authResolvedRef = useRef(false)
  const [initialServiceRefreshDone, setInitialServiceRefreshDone] = useState(false)
  const autoReleaseCheckRef = useRef(false)
  const savedServiceSlugRef = useRef('')
  const launchButtonAccentRef = useRef<string | null>(null)
  const previousAccountIdRef = useRef<string | null>(null)
  const snapshotReady = snapshot !== null
  const serviceAuthenticated = snapshot?.service.authenticated ?? false
  const latestUpdate = snapshot?.updates.latest ?? null
  const copy = snapshot ? getCopy(snapshot.language, snapshot.resolvedLanguage) : getCopy('system')
  const serviceLogContent = serviceLogViewer?.snapshot?.content ?? ''
  const serviceLogInputQuery = serviceLogViewer?.query ?? ''
  const serviceLogQuery = useDeferredValue(serviceLogInputQuery.trim())
  const serviceLogActiveMatchIndex = serviceLogViewer?.activeMatchIndex ?? 0
  const serviceLogViewerOpen = Boolean(serviceLogViewer)
  const serviceLogSearchCurrent =
    serviceLogSearch.query === serviceLogQuery &&
    serviceLogSearch.activeMatchIndex === serviceLogActiveMatchIndex
  const serviceLogMatchCount = serviceLogSearchCurrent ? serviceLogSearch.count : 0
  const serviceLogSearching = Boolean(serviceLogQuery) && (
    !serviceLogSearchCurrent || serviceLogSearch.searching
  )
  const serviceLogLineCount = useMemo(
    () => countLogLines(serviceLogContent),
    [serviceLogContent],
  )

  useEffect(() => {
    const query = serviceLogQuery
    const activeMatchIndex = serviceLogActiveMatchIndex
    if (!serviceLogContent || !query) {
      setServiceLogSearch(EMPTY_SERVICE_LOG_SEARCH)
      clearLogHighlight('slock-service-log-active')
      return
    }

    let cancelled = false
    setServiceLogSearch({
      query,
      activeMatchIndex,
      count: 0,
      activeStart: -1,
      activeEnd: -1,
      searching: true,
    })

    const complete = (result: Omit<ServiceLogSearchState, 'searching'>) => {
      if (!cancelled) {
        setServiceLogSearch({ ...result, searching: false })
      }
    }
    const timeout = window.setTimeout(() => {
      scanLogMatchesInChunks(serviceLogContent, query, activeMatchIndex, complete, () => cancelled)
    }, 120)

    return () => {
      cancelled = true
      window.clearTimeout(timeout)
    }
  }, [serviceLogActiveMatchIndex, serviceLogContent, serviceLogQuery])

  useEffect(() => {
    if (
      serviceLogSearch.searching ||
      !serviceLogSearch.query ||
      serviceLogSearch.activeStart < 0 ||
      serviceLogSearch.activeEnd <= serviceLogSearch.activeStart
    ) {
      clearLogHighlight('slock-service-log-active')
      return
    }

    const range = getLogTextRange(
      serviceLogContentRef.current,
      serviceLogSearch.activeStart,
      serviceLogSearch.activeEnd,
    )
    if (!range) {
      clearLogHighlight('slock-service-log-active')
      return
    }

    applyLogHighlight('slock-service-log-active', range)
    scrollLogRangeIntoView(range, serviceLogContentRef.current)
    return () => clearLogHighlight('slock-service-log-active')
  }, [
    serviceLogSearch.activeEnd,
    serviceLogSearch.activeStart,
    serviceLogSearch.query,
    serviceLogSearch.searching,
  ])

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
    let cancelled = false
    let unlisten: (() => void) | undefined

    void listen('desktop-auth-complete', () => {
      authResolvedRef.current = true
      void loadBootstrap(true).then((next) => {
        if (cancelled) {
          return
        }
        startTransition(() => setSnapshot(next))
        setBrowserLoginPending(false)
        setWorkspaceLaunchActive(false)
        setWorkspaceLaunchTarget(null)
      })
    }).then((cleanup) => {
      unlisten = cleanup
    })

    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    let hideTimer: number | undefined
    let unlisten: (() => void) | undefined

    void listen('desktop-auth-cancelled', () => {
      if (cancelled) {
        return
      }

      const restoreId = previousAccountIdRef.current
      previousAccountIdRef.current = null

      window.clearTimeout(hideTimer)
      setBrowserLoginPending(false)
      setBusyAction(null)
      setErrorMessage(null)

      if (restoreId) {
        void activateAccount(restoreId)
          .then((next) => {
            if (cancelled) return
            startTransition(() => setSnapshot(next))
            setWorkspaceLaunchActive(false)
            setWorkspaceLaunchTarget(null)
          })
          .catch(() => {
            if (cancelled) return
            setWorkspaceLaunchActive(true)
            setWorkspaceLaunchTarget(copy.loginInterrupted)
            hideTimer = window.setTimeout(() => {
              if (cancelled) return
              setWorkspaceLaunchActive(false)
              setWorkspaceLaunchTarget(null)
            }, AUTH_INTERRUPTED_HINT_MS)
          })
      } else {
        setWorkspaceLaunchActive(true)
        setWorkspaceLaunchTarget(copy.loginInterrupted)
        hideTimer = window.setTimeout(() => {
          if (cancelled) return
          setWorkspaceLaunchActive(false)
          setWorkspaceLaunchTarget(null)
        }, AUTH_INTERRUPTED_HINT_MS)
      }
    }).then((cleanup) => {
      unlisten = cleanup
    })

    return () => {
      cancelled = true
      window.clearTimeout(hideTimer)
      unlisten?.()
    }
  }, [copy.loginInterrupted])

  useEffect(() => {
    if (!browserLoginPending) {
      return
    }

    authResolvedRef.current = false
    let attempts = 0
    let cancelled = false
    const timer = window.setInterval(() => {
      if (authResolvedRef.current) {
        window.clearInterval(timer)
        return
      }

      attempts += 1
      void loadBootstrap(false)
        .then((next) => {
          if (cancelled || authResolvedRef.current) {
            return
          }
          if (next.service.authenticated) {
            authResolvedRef.current = true
            void loadBootstrap(true).then((refreshed) => {
              if (cancelled) return
              startTransition(() => setSnapshot(refreshed))
              setBrowserLoginPending(false)
              setWorkspaceLaunchActive(false)
              setWorkspaceLaunchTarget(null)
            })
          } else {
            startTransition(() => setSnapshot(next))
            if (attempts >= AUTH_POLL_MAX_ATTEMPTS) {
              setBrowserLoginPending(false)
              setWorkspaceLaunchActive(false)
              setWorkspaceLaunchTarget(null)
              setErrorMessage(copy.loginTimeout)
            }
          }
        })
        .catch((error) => {
          if (cancelled || authResolvedRef.current) {
            return
          }
          if (attempts >= AUTH_POLL_MAX_ATTEMPTS) {
            setBrowserLoginPending(false)
            setWorkspaceLaunchActive(false)
            setWorkspaceLaunchTarget(null)
            setErrorMessage(getErrorMessage(error))
          }
        })
    }, AUTH_POLL_INTERVAL_MS)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [browserLoginPending, copy.loginTimeout])

  useEffect(() => {
    savedServiceSlugRef.current = snapshot?.service.selectedServerSlug ?? ''
  }, [snapshot?.service.selectedServerSlug])

  useEffect(() => {
    if (!serviceAuthenticated || initialServiceRefreshRef.current) {
      return
    }

    initialServiceRefreshRef.current = true
    let cancelled = false
    setServiceRefreshPhase('catalog')
    void (async () => {
      try {
        const catalog = await refreshServiceServerCatalog()
        if (cancelled) {
          return
        }
        startTransition(() => setSnapshot(catalog))
        setServiceRefreshPhase('status')
        await waitForNextPaint()
        if (cancelled) {
          return
        }
        const status = await refreshServiceServerStatus()
        if (!cancelled) {
          startTransition(() => setSnapshot(status))
        }
      } catch (error) {
        if (!cancelled) {
          setErrorMessage(getErrorMessage(error))
        }
      } finally {
        if (!cancelled) {
          setServiceRefreshPhase(null)
          setInitialServiceRefreshDone(true)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [serviceAuthenticated])

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

  useEffect(() => {
    if (!serviceLogViewerOpen) {
      return
    }

    const timeout = window.setTimeout(() => {
      serviceLogSearchRef.current?.focus()
    }, 0)
    return () => window.clearTimeout(timeout)
  }, [serviceLogViewerOpen, serviceLogViewer?.serverSlug, serviceLogViewer?.loading])

  useEffect(() => {
    if (!serviceLogViewer) {
      return
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setServiceLogViewer(null)
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [serviceLogViewer])

  useEffect(() => {
    if (!newThemeDraft) {
      setNewThemeWheelOpen(false)
      return
    }

    const closeDraftOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (themePanelRef.current?.contains(target)) {
        return
      }
      if (themeDraftRef.current?.contains(target)) {
        return
      }
      setNewThemeDraft(null)
      setNewThemeWheelOpen(false)
    }

    document.addEventListener('pointerdown', closeDraftOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeDraftOnOutsidePointer)
  }, [newThemeDraft])

  useEffect(() => {
    if (!accountMenuOpen) {
      return
    }

    const closeAccountMenuOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (accountMenuRef.current?.contains(target)) {
        return
      }
      setAccountMenuOpen(false)
    }

    document.addEventListener('pointerdown', closeAccountMenuOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeAccountMenuOnOutsidePointer)
  }, [accountMenuOpen])

  useEffect(() => {
    if (!serverPanelOpen) {
      return
    }

    const closeServerPanelOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (serverPanelRef.current?.contains(target)) {
        return
      }
      setServerPanelOpen(false)
    }

    document.addEventListener('pointerdown', closeServerPanelOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeServerPanelOnOutsidePointer)
  }, [serverPanelOpen])

  useEffect(() => {
    if (!themePanelOpen) {
      return
    }

    const closeThemePanelOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (themePanelRef.current?.contains(target)) {
        return
      }
      setThemePanelOpen(false)
    }

    document.addEventListener('pointerdown', closeThemePanelOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeThemePanelOnOutsidePointer)
  }, [themePanelOpen])

  useEffect(() => {
    if (!stylePanelOpen) {
      return
    }

    const closeStylePanelOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (stylePanelRef.current?.contains(target)) {
        return
      }
      setStylePanelOpen(false)
    }

    document.addEventListener('pointerdown', closeStylePanelOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeStylePanelOnOutsidePointer)
  }, [stylePanelOpen])

  useEffect(() => {
    let unlisten: (() => void) | undefined

    void listen<MessageReminderToast>('slock-message-reminder', (event) => {
      setMessageReminder(event.payload)
      if (messageReminderTimerRef.current !== null) {
        window.clearTimeout(messageReminderTimerRef.current)
      }
      messageReminderTimerRef.current = window.setTimeout(() => {
        setMessageReminder(null)
        messageReminderTimerRef.current = null
      }, MESSAGE_REMINDER_TOAST_MS)
    }).then((cleanup) => {
      unlisten = cleanup
    })

    return () => {
      unlisten?.()
      if (messageReminderTimerRef.current !== null) {
        window.clearTimeout(messageReminderTimerRef.current)
        messageReminderTimerRef.current = null
      }
    }
  }, [])

  useEffect(() => {
    if (!agentCardTarget) {
      return
    }

    const closeAgentCardOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (agentCardRef.current?.contains(target)) {
        return
      }
      setAgentCardTarget(null)
    }

    document.addEventListener('pointerdown', closeAgentCardOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeAgentCardOnOutsidePointer)
  }, [agentCardTarget])

  // Fetch dashboard data when selected server changes
  useEffect(() => {
    if (!snapshot?.service.selectedServerSlug || !snapshot.service.authenticated) {
      setDashboardData(null)
      setDashboardError(null)
      return
    }

    // Wait for initial service refresh to populate cached_servers
    if (!initialServiceRefreshDone) {
      return
    }

    const serverSlug = snapshot.service.selectedServerSlug
    let cancelled = false

    async function loadDashboard() {
      setDashboardLoading(true)
      setDashboardError(null)
      try {
        const data = await fetchDashboard(serverSlug)
        if (!cancelled) {
          setDashboardData(data)
          setDashboardError(null)
        }
      } catch (err) {
        if (!cancelled) {
          setDashboardData(null)
          setDashboardError(
            err instanceof Error ? err.message : 'Failed to load dashboard',
          )
        }
      } finally {
        if (!cancelled) {
          setDashboardLoading(false)
        }
      }
    }

    void loadDashboard()
    return () => { cancelled = true }
  }, [snapshot?.service.selectedServerSlug, snapshot?.service.authenticated, initialServiceRefreshDone])

  useEffect(() => {
    if (
      !snapshotReady ||
      autoReleaseCheckRef.current ||
      (serviceAuthenticated && !initialServiceRefreshDone)
    ) {
      return
    }

    if (latestUpdate) {
      autoReleaseCheckRef.current = true
      setReleaseState({
        loading: false,
        installing: false,
        error: null,
        latest: latestUpdate,
      })
      return
    }

    autoReleaseCheckRef.current = true
    let cancelled = false
    void checkDesktopUpdate()
      .then((latest) => {
        if (cancelled) {
          return
        }
        setReleaseState({
          loading: false,
          installing: false,
          error: null,
          latest,
        })
      })
      .catch((error) => {
        if (cancelled) {
          return
        }
        console.warn('[Slock Desktop] automatic update check failed', error)
      })

    return () => {
      cancelled = true
    }
  }, [snapshotReady, serviceAuthenticated, initialServiceRefreshDone, latestUpdate])

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

  async function handleThemeStyleChange(styleId: string) {
    try {
      setBusyAction(`style:${styleId}`)
      setErrorMessage(null)
      const next = await updateThemeStyle(styleId)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  function handleExportThemeStyle(style: ThemeStyleDefinition | null | undefined) {
    if (!style) {
      return
    }
    exportThemeStyleFile(style)
  }

  async function handleImportThemeStyleFile(event: ReactChangeEvent<HTMLInputElement>) {
    const file = event.currentTarget.files?.[0]
    event.currentTarget.value = ''
    if (!file) {
      return
    }

    try {
      setBusyAction('import-style')
      setErrorMessage(null)
      const parsed = JSON.parse(await file.text()) as unknown
      const config = readThemeStyleConfig(parsed)
      const next = await importThemeStyle(config)
      startTransition(() => setSnapshot(next))
    } catch {
      setErrorMessage(copy.themeImportInvalid)
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
    setNewThemeDraft(createNewThemeDraft())
    setNewThemeWheelOpen(true)
  }

  function cancelNewTheme() {
    setNewThemeDraft(null)
    setNewThemeWheelOpen(false)
  }

  function updateNewThemeAccent(accent: string) {
    setNewThemeDraft((current) =>
      current ? syncNewThemeDraftAccent(current, accent) : current,
    )
  }

  function handleNewThemeHexChange(value: string) {
    setNewThemeDraft((current) => {
      if (!current) {
        return current
      }
      const normalized = normalizeHexColor(value)
      if (!normalized) {
        return { ...current, hexInput: value.toUpperCase() }
      }
      return syncNewThemeDraftAccent({ ...current, hexInput: value.toUpperCase() }, normalized)
    })
  }

  function handleNewThemeRgbChange(channel: RgbChannel, value: string) {
    const nextValue = sanitizeRgbInput(value)
    setNewThemeDraft((current) => {
      if (!current) {
        return current
      }

      const rgbInput = { ...current.rgbInput, [channel]: nextValue }
      const rgb = parseRgbInput(rgbInput)
      if (!rgb) {
        return { ...current, rgbInput }
      }

      return syncNewThemeDraftAccent(
        { ...current, rgbInput },
        rgbToHex(rgb.r, rgb.g, rgb.b),
      )
    })
  }

  function handleNewThemeWheelPointer(event: ReactPointerEvent<HTMLDivElement>) {
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    updateNewThemeAccent(getAccentFromWheelPointer(event.clientX, event.clientY, event.currentTarget))
  }

  function handleNewThemeWheelMove(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.buttons !== 1) {
      return
    }
    updateNewThemeAccent(getAccentFromWheelPointer(event.clientX, event.clientY, event.currentTarget))
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
      setNewThemeWheelOpen(false)
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

  function handleMessageReminderDismiss() {
    setMessageReminder(null)
    if (messageReminderTimerRef.current !== null) {
      window.clearTimeout(messageReminderTimerRef.current)
      messageReminderTimerRef.current = null
    }
  }

  async function handleMessageReminderOpen(reminder: MessageReminderToast) {
    handleMessageReminderDismiss()
    await handleWorkspaceOpen(reminder.serverSlug)
  }

  async function runBrowserAuthAction(
    action: string,
    authFn: () => Promise<BootstrapPayload>,
  ) {
    try {
      setBusyAction(action)
      setWorkspaceLaunchActive(true)
      setWorkspaceLaunchTarget(copy.browserLoginPending)
      setBrowserLoginPending(true)
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await authFn()
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setBrowserLoginPending(false)
      setWorkspaceLaunchActive(false)
      setWorkspaceLaunchTarget(null)
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleLoginOpen() {
    await runBrowserAuthAction('login', openLogin)
  }

  async function handleSwitchAccount() {
    setAccountMenuOpen(false)
    previousAccountIdRef.current = snapshot?.service.account?.id ?? null
    await runBrowserAuthAction('switch-account', switchAccount)
  }

  async function handleSavedAccountSelect(accountId: string) {
    try {
      setBusyAction(`account:${accountId}`)
      setAccountMenuOpen(false)
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await activateAccount(accountId)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleForgetAccount(accountId: string) {
    try {
      setBusyAction(`forget:${accountId}`)
      setErrorMessage(null)
      const next = await forgetAccount(accountId)
      startTransition(() => setSnapshot(next))
    } catch (error) {
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
      const next = await refreshServiceServerStatus()
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setServiceRefreshPhase(null)
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

  async function handleServiceLogOpen(serverSlug: string) {
    const server =
      snapshot?.service.servers.find((item) => item.slug === serverSlug) ?? null
    const serverName = server?.name ?? serverSlug
    const query = serviceLogViewer?.serverSlug === serverSlug ? serviceLogViewer.query : ''
    const preservedRange = serviceLogViewer?.serverSlug === serverSlug
    const defaultRange = getDefaultServiceLogRange()
    const rangeStart = preservedRange ? serviceLogViewer.rangeStart : defaultRange.start
    const rangeEnd = preservedRange ? serviceLogViewer.rangeEnd : defaultRange.end
    const rangePresetMs = preservedRange
      ? serviceLogViewer.rangePresetMs
      : DEFAULT_SERVICE_LOG_RANGE_MS

    setServiceLogViewer({
      loading: true,
      snapshot: null,
      serverSlug,
      serverName,
      query,
      rangeStart,
      rangeEnd,
      rangePresetMs,
      activeMatchIndex: 0,
      error: null,
    })

    try {
      setErrorMessage(null)
      const next = await openServiceLog(serverSlug, {
        fromEpochMs: datetimeLocalToEpochMs(rangeStart),
        toEpochMs: datetimeLocalToEpochMs(rangeEnd),
      })
      setServiceLogViewer((current) => {
        if (!current || current.serverSlug !== serverSlug) {
          return current
        }
        return {
          ...current,
          loading: false,
          snapshot: next,
          serverName,
          activeMatchIndex: 0,
          error: null,
        }
      })
    } catch (error) {
      const message = getErrorMessage(error)
      setServiceLogViewer((current) =>
        current && current.serverSlug === serverSlug
          ? { ...current, loading: false, error: message }
          : current,
      )
    }
  }

  function handleServiceLogQueryChange(query: string) {
    setServiceLogViewer((current) =>
      current ? { ...current, query, activeMatchIndex: 0 } : current,
    )
  }

  function handleServiceLogRangePartChange(
    field: 'rangeStart' | 'rangeEnd',
    part: 'date' | 'time',
    value: string,
  ) {
    setServiceLogViewer((current) =>
      current
        ? {
            ...current,
            [field]: updateDatetimeLocalPart(current[field], part, value),
            rangePresetMs: null,
          }
        : current,
    )
  }

  function handleServiceLogRangePresetChange(durationMs: number) {
    const range = getServiceLogRangeForDuration(durationMs)
    setServiceLogViewer((current) =>
      current
        ? {
            ...current,
            rangeStart: range.start,
            rangeEnd: range.end,
            rangePresetMs: durationMs,
          }
        : current,
    )
    if (serviceLogViewer?.serverSlug) {
      void handleServiceLogOpenWithRange(
        serviceLogViewer.serverSlug,
        range.start,
        range.end,
        durationMs,
      )
    }
  }

  async function handleServiceLogOpenWithRange(
    serverSlug: string,
    rangeStart: string,
    rangeEnd: string,
    rangePresetMs: number | null = serviceLogViewer?.rangePresetMs ?? null,
  ) {
    setServiceLogViewer((current) =>
      current && current.serverSlug === serverSlug
        ? {
            ...current,
            loading: true,
            snapshot: null,
            rangeStart,
            rangeEnd,
            rangePresetMs,
            error: null,
          }
        : current,
    )
    try {
      const next = await openServiceLog(serverSlug, {
        fromEpochMs: datetimeLocalToEpochMs(rangeStart),
        toEpochMs: datetimeLocalToEpochMs(rangeEnd),
      })
      setServiceLogViewer((current) =>
        current && current.serverSlug === serverSlug
          ? {
              ...current,
              loading: false,
              snapshot: next,
              rangePresetMs,
              activeMatchIndex: 0,
              error: null,
            }
          : current,
      )
    } catch (error) {
      const message = getErrorMessage(error)
      setServiceLogViewer((current) =>
        current && current.serverSlug === serverSlug
          ? { ...current, loading: false, error: message }
          : current,
      )
    }
  }

  function handleServiceLogMatchStep(direction: number) {
    if (serviceLogSearching || serviceLogMatchCount === 0) {
      return
    }

    setServiceLogViewer((current) => {
      if (!current?.snapshot) {
        return current
      }
      return {
        ...current,
        activeMatchIndex:
          (current.activeMatchIndex + direction + serviceLogMatchCount) %
          serviceLogMatchCount,
      }
    })
  }

  function handleServiceLogClose() {
    setServiceLogViewer(null)
  }

  async function handleSelectedServiceToggle() {
    if (!snapshot) {
      return
    }

    const selected =
      snapshot.service.servers.find(
        (server) => server.slug === snapshot.service.selectedServerSlug,
      ) ??
      snapshot.service.servers.find((server) => server.selected) ??
      snapshot.service.servers[0] ??
      null
    const selectedServerSlug = selected?.slug ?? snapshot.service.selectedServerSlug

    if (!selectedServerSlug) {
      setErrorMessage(copy.selectedServerPlaceholder)
      return
    }

    const running = isSelectedServiceRunning(snapshot.service, selectedServerSlug)

    try {
      setBusyAction(running ? 'stop-service' : 'start-service')
      setErrorMessage(null)
      await waitForNextPaint()
      const next = running
        ? await stopService(selectedServerSlug)
        : await startService(selectedServerSlug)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleAgentCardOpen(agent: DashboardAgent) {
    if (agentCardTarget?.id === agent.id) {
      setAgentCardTarget(null)
      return
    }
    setAgentCardTarget(agent)
    setAgentCardActivity([])
    setAgentCardLoading(true)
    try {
      const serverSlug = snapshot?.service.selectedServerSlug
      if (serverSlug) {
        const activity = await fetchAgentActivity(serverSlug, agent.id)
        setAgentCardActivity(activity.slice(0, 5))
      }
    } catch {
      // Activity load failure is non-critical
    } finally {
      setAgentCardLoading(false)
    }
  }

  async function handleAgentStop(agent: DashboardAgent) {
    const serverSlug = snapshot?.service.selectedServerSlug
    if (!serverSlug) { return }
    try {
      setAgentCardAction('stop')
      await stopAgent(serverSlug, agent.id)
      // Refresh dashboard to get updated agent status
      const data = await fetchDashboard(serverSlug)
      startTransition(() => setDashboardData(data))
      setAgentCardTarget(null)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setAgentCardAction(null)
    }
  }

  async function handleAgentStart(agent: DashboardAgent) {
    const serverSlug = snapshot?.service.selectedServerSlug
    if (!serverSlug) { return }
    try {
      setAgentCardAction('start')
      await startAgent(serverSlug, agent.id)
      // Refresh dashboard to get updated agent status
      const data = await fetchDashboard(serverSlug)
      startTransition(() => setDashboardData(data))
      setAgentCardTarget(null)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setAgentCardAction(null)
    }
  }

  async function handleAgentRestart(agent: DashboardAgent) {
    const serverSlug = snapshot?.service.selectedServerSlug
    if (!serverSlug) { return }
    try {
      setAgentCardAction('restart')
      await stopAgent(serverSlug, agent.id)
      await startAgent(serverSlug, agent.id)
      // Refresh dashboard to get updated agent status
      const data = await fetchDashboard(serverSlug)
      startTransition(() => setDashboardData(data))
      setAgentCardTarget(null)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setAgentCardAction(null)
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
    return (
      <main className="loading-shell">
        <SlockBrandMark className="loading-mark" />
        <SpinnerIcon />
        <p className="eyebrow">{copy.appBootingTitle}</p>
      </main>
    )
  }

  const activeTheme =
    snapshot.themes.find((theme) => theme.id === snapshot.colorScheme) ??
    snapshot.themes[0]
  const activeStyle =
    snapshot.themeStyles.find((style) => style.id === snapshot.styleScheme) ??
    snapshot.themeStyles[0]
  const selectedThemeAccent = activeTheme.accent
  const stackButtonLabel = snapshot.workspaceOpen ? copy.focusSlock : copy.openSlock
  const selectedServiceServer =
    snapshot.service.servers.find(
      (server) => server.slug === snapshot.service.selectedServerSlug,
    ) ??
    snapshot.service.servers.find((server) => server.selected) ??
    null
  const savedServiceSlug = snapshot.service.selectedServerSlug.trim()
  const selectedServiceSlug = selectedServiceServer?.slug ?? savedServiceSlug
  const normalizedServerQuery = serverQuery.trim().toLowerCase()
  const filteredServiceServers = normalizedServerQuery
    ? snapshot.service.servers.filter((server) => {
        const machineName = server.machineName ?? ''
        return `${server.name} ${server.slug} ${machineName}`
          .toLowerCase()
          .includes(normalizedServerQuery)
      })
    : snapshot.service.servers
  const selectedServiceRunning = isSelectedServiceRunning(
    snapshot.service,
    selectedServiceSlug,
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
    busyAction === 'login' ||
    busyAction === 'switch-account' ||
    busyAction === 'refresh-service' ||
    workspaceLaunchActive ||
    Boolean(workspaceLaunchTarget) ||
    Boolean(busyAction?.startsWith('account:')) ||
    Boolean(busyAction?.startsWith('select-service:'))
  const serviceToggleBusy = busyAction === 'start-service' || busyAction === 'stop-service'
  const serviceToggleLabel =
    busyAction === 'start-service'
      ? copy.startingService
      : busyAction === 'stop-service'
        ? copy.closingServer
        : selectedServiceRunning
          ? copy.closeServer
          : copy.startService
  const workspaceLaunching =
    busyAction === 'workspace' ||
    busyAction === 'login' ||
    busyAction === 'switch-account' ||
    workspaceLaunchActive ||
    Boolean(workspaceLaunchTarget) ||
    snapshot.workspaceOpen
  const launchButtonAccent =
    workspaceLaunching || busyAction === 'workspace'
      ? (launchButtonAccentRef.current ?? selectedThemeAccent)
      : selectedThemeAccent
  const shellStyle = {
    ...buildShellStyle(activeTheme),
    '--launch-accent': launchButtonAccent,
  } as CSSProperties
  const activeIsOriginal = snapshot.styleScheme === 'original' || !snapshot.styleScheme
  const releaseUpdateAvailable = Boolean(releaseState.latest?.available)
  const releaseStatusLabel = releaseState.loading
    ? copy.checkingRelease
    : releaseState.installing
      ? copy.installingDesktopUpdate
      : releaseState.latest?.available
        ? copy.updateAvailable
        : ''
  const releaseStatusTitle =
    releaseState.error ??
    (releaseState.latest?.version
      ? `v${releaseState.latest.version} — ${releaseStatusLabel || copy.current}`
      : releaseStatusLabel || copy.notChecked)
  const accountEmailLabel = getAccountEmailLabel(snapshot.service.account, copy)
  const savedAccounts = snapshot.service.accounts
  const currentAccountId = snapshot.service.account?.id ?? ''
  const serviceLogStatusLabel = serviceLogQuery
    ? serviceLogSearching
      ? copy.serverLogSearching
      : serviceLogMatchCount > 0
      ? `${Math.min(serviceLogViewer?.activeMatchIndex ?? 0, serviceLogMatchCount - 1) + 1}/${serviceLogMatchCount}`
      : copy.serverLogNoMatches
    : `${serviceLogLineCount} ${copy.serverLogLines}`

  return (
    <main
      className="studio-shell"
      data-mode={activeTheme.mode}
      style={shellStyle}
      aria-busy={workspaceLaunching}
    >
      <header className="tauri-titlebar">
        <div className="titlebar-account" ref={accountMenuRef}>
          {snapshot.service.authenticated ? (
            <>
              <button
                type="button"
                className="titlebar-avatar-button"
                onClick={() => setAccountMenuOpen((open) => !open)}
                disabled={busyAction === 'switch-account'}
                title={accountEmailLabel}
                aria-expanded={accountMenuOpen}
                aria-label={accountEmailLabel}
              >
                <AccountAvatar account={snapshot.service.account} />
              </button>
              {accountMenuOpen ? (
                <div
                  className="account-menu"
                  role="menu"
                  aria-label={copy.switchAccount}
                >
                  {savedAccounts.map((account) => {
                    const selected = account.id === currentAccountId
                    const label = getAccountEmailLabel(account, copy)
                    return (
                      <div
                        key={account.id}
                        className={`account-menu-item-wrap${selected ? ' selected' : ''}`}
                      >
                        <button
                          type="button"
                          className="account-menu-item-main"
                          role="menuitem"
                          onClick={() => {
                            if (!selected) {
                              void handleSavedAccountSelect(account.id)
                            }
                          }}
                          disabled={selected || busyAction === `account:${account.id}`}
                        >
                          <AccountAvatar account={account} />
                          <span className="account-menu-copy">
                            <span>{label}</span>
                            {selected ? <span>{copy.currentAccount}</span> : null}
                          </span>
                        </button>
                        <button
                          type="button"
                          className="account-menu-forget"
                          title={copy.forgetAccount}
                          aria-label={copy.forgetAccount}
                          disabled={busyAction === `forget:${account.id}`}
                          onClick={(e) => {
                            e.stopPropagation()
                            void handleForgetAccount(account.id)
                          }}
                        >
                          ×
                        </button>
                      </div>
                    )
                  })}
                  <button
                    type="button"
                    className="account-menu-item add"
                    role="menuitem"
                    onClick={() => void handleSwitchAccount()}
                  >
                    <span className="account-menu-add-mark">+</span>
                    <span>{copy.addAccount}</span>
                  </button>
                </div>
              ) : null}
            </>
          ) : (
            <button
              type="button"
              className="titlebar-signin-button"
              onClick={handleLoginOpen}
              disabled={workspaceLaunching}
              title={copy.signIn}
            >
              <span>{copy.signIn}</span>
            </button>
          )}
        </div>

        <div className="titlebar-server" ref={serverPanelRef}>
          <button
            type="button"
            className="titlebar-server-button"
            onClick={() => setServerPanelOpen((open) => !open)}
            disabled={!snapshot.service.authenticated}
            aria-expanded={serverPanelOpen}
            title={selectedServiceServer?.name ?? copy.selectedServerPlaceholder}
          >
            <span className="titlebar-server-name">
              {selectedServiceServer?.name ?? copy.selectedServerPlaceholder}
            </span>
            {selectedServiceRunning ? (
              <span className="titlebar-server-dot running" aria-hidden="true" />
            ) : null}
          </button>
          {serverPanelOpen ? (
            <div className="titlebar-server-panel" role="listbox" aria-label={copy.service}>
              <div className="titlebar-server-panel-head">
                <div className="titlebar-server-panel-actions">
                  <span className="server-count-pill">
                    {normalizedServerQuery
                      ? `${filteredServiceServers.length}/${snapshot.service.servers.length}`
                      : `${snapshot.service.servers.length}`}
                  </span>
                  <button
                    className={`service-toggle-button${selectedServiceRunning ? ' running' : ''}`}
                    type="button"
                    onClick={handleSelectedServiceToggle}
                    disabled={!selectedServiceSlug || serviceActionBusy}
                    title={serviceToggleLabel}
                    aria-label={serviceToggleLabel}
                  >
                    <ServiceActionIcon
                      type={selectedServiceRunning ? 'stop' : 'start'}
                      busy={serviceToggleBusy}
                    />
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
                  const serverSelectionDisabled =
                    busyAction?.startsWith('select-service:') ||
                    busyAction === 'start-service' ||
                    busyAction === 'workspace' ||
                    busyAction === 'stop-service' ||
                    busyAction === 'refresh-service'
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
                    <div
                      key={server.id}
                      className={`service-server-row${selected ? ' selected' : ''}${running ? ' running' : ''}`}
                      data-disabled={serverSelectionDisabled}
                    >
                      <button
                        className="service-server-select-button"
                        type="button"
                        aria-pressed={selected}
                        disabled={serverSelectionDisabled}
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
                      <button
                        className="icon-action-button compact service-server-log-button"
                        type="button"
                        onClick={() => handleServiceLogOpen(server.slug)}
                        aria-label={`${copy.openServerLog}: ${server.name}`}
                        title={copy.openServerLog}
                      >
                        <LogsIcon />
                      </button>
                    </div>
                  )
                })}
                {snapshot.service.servers.length > 0 && filteredServiceServers.length === 0 ? (
                  <p className="inline-note">{copy.noMatchingServers}</p>
                ) : null}
              </div>
            </div>
          ) : null}
        </div>

        <div className="tauri-titlebar-drag" data-tauri-drag-region />

        <button
          type="button"
          className={`titlebar-icon-button${selectedServiceRunning ? ' running' : ''}`}
          onClick={handleSelectedServiceToggle}
          disabled={!selectedServiceSlug || serviceActionBusy}
          title={serviceToggleLabel}
          aria-label={serviceToggleLabel}
        >
          <ServiceActionIcon
            type={selectedServiceRunning ? 'stop' : 'start'}
            busy={serviceToggleBusy}
          />
        </button>

        <button
          type="button"
          className="titlebar-icon-button"
          onClick={() => {
            if (selectedServiceSlug) {
              void handleServiceLogOpen(selectedServiceSlug)
            }
          }}
          disabled={!selectedServiceSlug}
          title={copy.openServerLog}
          aria-label={copy.openServerLog}
        >
          <LogsIcon />
        </button>

        <div className="titlebar-style" ref={stylePanelRef}>
          <button
            type="button"
            className="titlebar-icon-button"
            onClick={() => setStylePanelOpen((open) => !open)}
            aria-expanded={stylePanelOpen}
            title={copy.themeStyle}
            aria-label={copy.themeStyle}
          >
            <StyleIcon />
          </button>
          {stylePanelOpen ? (
            <div className="titlebar-style-panel" aria-label={copy.themeStyle}>
              <div className="control-card-head">
                <p className="eyebrow">{copy.themeStyle}</p>
                <span className="theme-style-actions">
                  <button
                    className="text-action-button"
                    type="button"
                    onClick={() => styleImportInputRef.current?.click()}
                    disabled={busyAction === 'import-style'}
                  >
                    {copy.themeImportStyle}
                  </button>
                  <button
                    className="text-action-button"
                    type="button"
                    onClick={() => handleExportThemeStyle(activeStyle)}
                    disabled={!activeStyle}
                  >
                    {copy.themeExportStyle}
                  </button>
                </span>
              </div>
              <input
                ref={styleImportInputRef}
                className="sr-only"
                type="file"
                accept="application/json,.json"
                onChange={(event) => void handleImportThemeStyleFile(event)}
              />
              <ul className="theme-rail theme-style-rail" role="radiogroup" aria-label={copy.themeStyle}>
                {snapshot.themeStyles.map((style) => (
                  <li key={style.id}>
                    <ThemeStyleRow
                      style={style}
                      name={getThemeStyleName(style, copy)}
                      summary={getThemeStyleSummary(style, copy)}
                      selected={style.id === snapshot.styleScheme || (style.id === 'original' && activeIsOriginal)}
                      busy={busyAction === `style:${style.id}`}
                      onSelect={() => handleThemeStyleChange(style.id)}
                    />
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>

        <div className="titlebar-theme" ref={themePanelRef}>
          <button
            type="button"
            className="titlebar-theme-button"
            onClick={() => setThemePanelOpen((open) => !open)}
            aria-expanded={themePanelOpen}
            aria-label={copy.themeColor}
            title={copy.themeColor}
            style={{ '--current-accent': selectedThemeAccent } as CSSProperties}
          >
            <span className="titlebar-theme-swatch" aria-hidden="true" />
          </button>
          {themePanelOpen ? (
            <div className="titlebar-theme-panel" aria-label={copy.themeColor}>
              <div className="control-card-head">
                <p className="eyebrow">{copy.themeColor}</p>
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
                {snapshot.themes.map((theme) => {
                  const customTheme = snapshot.customThemes.find((item) => item.id === theme.id)
                  const selected = theme.id === snapshot.colorScheme
                  const isRenaming = theme.id === renamingThemeId
                  const builtIn = !customTheme
                  const swatch = customTheme?.accent ?? DEFAULT_NEW_THEME_ACCENT
                  return (
                    <li key={theme.id}>
                      <ThemeRow
                        themeId={theme.id}
                        swatch={swatch}
                        name={builtIn ? copy.themeDefaultColorName : (customTheme?.name ?? theme.name)}
                        summary={builtIn ? copy.themeDefaultColorSummary : swatch.toUpperCase()}
                        selected={selected}
                        busy={busyAction === `theme:${theme.id}`}
                        locked={builtIn}
                        lockedLabel={copy.themeBuiltIn}
                        renaming={isRenaming}
                        renameDraft={renameDraft}
                        onRenameDraftChange={setRenameDraft}
                        renameInputRef={renameInputRef}
                        onCommitRename={() => void commitRename()}
                        onCancelRename={cancelRename}
                        onSelect={() => handleThemeChange(theme.id)}
                        onStartRename={customTheme ? () => startRename(customTheme) : undefined}
                        onAccentChange={customTheme ? (value) => void handleAccentChange(theme.id, value) : undefined}
                        onDelete={customTheme ? () => void handleDeleteTheme(theme.id) : undefined}
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
              </ul>

            {newThemeDraft ? (
              <div
                ref={themeDraftRef}
                className="theme-draft-row theme-draft-floating"
                role="dialog"
                aria-label={copy.themeNewTitle}
              >
                <div
                  className="theme-draft-accent"
                  style={{ '--custom-accent': newThemeDraft.accent } as CSSProperties}
                >
                  <button
                    className={`accent-wheel${newThemeWheelOpen ? ' expanded' : ''}`}
                    type="button"
                    onClick={() => setNewThemeWheelOpen((open) => !open)}
                    aria-label={copy.customThemeAccentAria}
                    aria-expanded={newThemeWheelOpen}
                  >
                    <span aria-hidden="true" />
                  </button>
                  {newThemeWheelOpen ? (
                    <div className="accent-wheel-popover">
                      <div
                        className="accent-wheel-large"
                        role="slider"
                        tabIndex={0}
                        aria-label={copy.customThemeAccentAria}
                        aria-valuetext={newThemeDraft.hexInput}
                        style={getAccentWheelMarkerStyle(newThemeDraft.accent)}
                        onPointerDown={handleNewThemeWheelPointer}
                        onPointerMove={handleNewThemeWheelMove}
                      >
                        <span className="accent-wheel-marker" aria-hidden="true" />
                      </div>
                    </div>
                  ) : null}
                </div>
                <div className="theme-draft-fields">
                  <div className="theme-color-picker-label">{copy.customThemeAccent}</div>
                  <div className="theme-preset-row" aria-label={copy.customThemeAccentAria}>
                    {THEME_ACCENT_PRESETS.map((accent) => {
                      const selected = normalizeHexColor(accent) === newThemeDraft.accent
                      return (
                        <button
                          key={accent}
                          className={`theme-preset-swatch${selected ? ' selected' : ''}`}
                          type="button"
                          style={{ '--preset-accent': accent } as CSSProperties}
                          onClick={() => updateNewThemeAccent(accent)}
                          aria-label={accent.toUpperCase()}
                          title={accent.toUpperCase()}
                        />
                      )
                    })}
                  </div>
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
                  <div className="theme-color-inputs">
                    <label className="theme-hex-input">
                      <span>HEX</span>
                      <input
                        value={newThemeDraft.hexInput}
                        onChange={(event) => handleNewThemeHexChange(event.target.value)}
                        inputMode="text"
                        spellCheck={false}
                        aria-label="HEX"
                      />
                    </label>
                    {(['r', 'g', 'b'] as const).map((channel) => (
                      <label key={channel} className="theme-rgb-input">
                        <span>{channel.toUpperCase()}</span>
                        <input
                          value={newThemeDraft.rgbInput[channel]}
                          onChange={(event) =>
                            handleNewThemeRgbChange(channel, event.target.value)
                          }
                          inputMode="numeric"
                          aria-label={channel.toUpperCase()}
                        />
                      </label>
                    ))}
                  </div>
                </div>
                <div className="theme-draft-actions">
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
              </div>
            ) : null}
            </div>
          ) : null}
        </div>

        <div className="titlebar-settings" aria-label={`${copy.mode} / ${copy.language}`}>
          <button
            type="button"
            className="titlebar-cycle-button"
            title={copy[THEME_MODES.find((m) => m.id === snapshot.appearanceMode)?.labelKey ?? 'modeSystem']}
            onClick={() => {
              const currentIndex = THEME_MODES.findIndex((m) => m.id === snapshot.appearanceMode)
              const nextIndex = (currentIndex + 1) % THEME_MODES.length
              void handleThemeModeChange(THEME_MODES[nextIndex].id)
            }}
            disabled={busyAction?.startsWith('mode:')}
          >
            <OptionIcon type={THEME_MODES.find((m) => m.id === snapshot.appearanceMode)?.icon ?? 'display'} />
          </button>
          <button
            type="button"
            className="titlebar-cycle-button"
            title={copy[LANGUAGE_OPTIONS.find((l) => l.id === snapshot.language)?.labelKey ?? 'languageSystem']}
            onClick={() => {
              const currentIndex = LANGUAGE_OPTIONS.findIndex((l) => l.id === snapshot.language)
              const nextIndex = (currentIndex + 1) % LANGUAGE_OPTIONS.length
              void handleLanguageChange(LANGUAGE_OPTIONS[nextIndex].id)
            }}
            disabled={busyAction?.startsWith('language:')}
          >
            <OptionIcon type={LANGUAGE_OPTIONS.find((l) => l.id === snapshot.language)?.icon ?? 'globe'} />
          </button>
        </div>
        {snapshot.workspaceOpen ? (
          <span className="status-pill live">{copy.workspaceActive}</span>
        ) : null}
        <span
          className={`status-chip titlebar-version${releaseUpdateAvailable ? ' warm' : ''}${releaseState.error ? ' error' : ''}`}
          title={releaseStatusTitle}
        >
          v{snapshot.updates.currentVersion}
          {releaseStatusLabel ? ` · ${releaseStatusLabel}` : ''}
        </span>
        {releaseUpdateAvailable ? (
          <button
            type="button"
            className="titlebar-update-button accent"
            onClick={handleDesktopUpdateInstall}
            disabled={releaseState.installing || releaseState.loading}
          >
            {releaseState.installing ? <SpinnerIcon /> : null}
            <span>
              {releaseState.installing
                ? copy.installingDesktopUpdate
                : copy.installDesktopUpdate}
            </span>
          </button>
        ) : null}
      </header>

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

        {dashboardData ? (
          <section className="dashboard" aria-label={copy.dashboardLabel}>
            {dashboardData.warnings && dashboardData.warnings.length > 0 ? (
              <p className="dashboard-warning inline-note">{copy.dashboardPartialError ?? 'Some data failed to load'}</p>
            ) : null}
            <div className="dashboard-stats">
              <div className="dashboard-stat-card">
                <span className="dashboard-stat-value">
                  {dashboardData.channels.filter((ch) => !ch.isArchived).length}
                </span>
                <span className="dashboard-stat-label">{copy.dashboardChannels ?? 'Channels'}</span>
              </div>
              <div className="dashboard-stat-card">
                <span className="dashboard-stat-value">
                  {dashboardData.unread.reduce((sum, u) => sum + u.unreadCount, 0)}
                </span>
                <span className="dashboard-stat-label">{copy.dashboardUnread ?? 'Unread'}</span>
              </div>
              <div className="dashboard-stat-card">
                <span className="dashboard-stat-value">{dashboardData.tasks.length}</span>
                <span className="dashboard-stat-label">{copy.dashboardTasks ?? 'Tasks'}</span>
              </div>
              <div className="dashboard-stat-card">
                <span className="dashboard-stat-value">
                  {dashboardData.agents.filter((a) => a.status !== 'offline').length}/{dashboardData.agents.length}
                </span>
                <span className="dashboard-stat-label">{copy.dashboardAgents ?? 'Agents'}</span>
              </div>
            </div>

            <div className="dashboard-panels">
              <div className="dashboard-panel">
                <p className="eyebrow">{copy.dashboardTaskStatus ?? 'Task Status'}</p>
                <div className="dashboard-task-bar">
                  {(() => {
                    const total = dashboardData.tasks.length || 1
                    const todo = dashboardData.tasks.filter((t) => t.status === 'todo').length
                    const inProgress = dashboardData.tasks.filter((t) => t.status === 'in_progress').length
                    const inReview = dashboardData.tasks.filter((t) => t.status === 'in_review').length
                    const done = dashboardData.tasks.filter((t) => t.status === 'done').length
                    return (
                      <>
                        {todo > 0 ? <span className="task-bar-segment todo" style={{ flex: todo / total }} title={`${copy.taskStatusTodo}: ${todo}`} /> : null}
                        {inProgress > 0 ? <span className="task-bar-segment in-progress" style={{ flex: inProgress / total }} title={`${copy.taskStatusInProgress}: ${inProgress}`} /> : null}
                        {inReview > 0 ? <span className="task-bar-segment in-review" style={{ flex: inReview / total }} title={`${copy.taskStatusInReview}: ${inReview}`} /> : null}
                        {done > 0 ? <span className="task-bar-segment done" style={{ flex: done / total }} title={`${copy.taskStatusDone}: ${done}`} /> : null}
                      </>
                    )
                  })()}
                </div>
                <div className="dashboard-task-legend">
                  <span className="task-legend-item"><span className="task-dot todo" />{copy.taskStatusTodo} {dashboardData.tasks.filter((t) => t.status === 'todo').length}</span>
                  <span className="task-legend-item"><span className="task-dot in-progress" />{copy.taskStatusInProgress} {dashboardData.tasks.filter((t) => t.status === 'in_progress').length}</span>
                  <span className="task-legend-item"><span className="task-dot in-review" />{copy.taskStatusInReview} {dashboardData.tasks.filter((t) => t.status === 'in_review').length}</span>
                  <span className="task-legend-item"><span className="task-dot done" />{copy.taskStatusDone} {dashboardData.tasks.filter((t) => t.status === 'done').length}</span>
                </div>
              </div>

              <div className="dashboard-panel">
                <p className="eyebrow">{copy.dashboardAgentStatus ?? 'Agents'}</p>
                <div className="dashboard-agent-list">
                  {dashboardData.agents.map((agent) => (
                    <div key={agent.id} className="dashboard-agent-row" ref={agentCardTarget?.id === agent.id ? agentCardRef : undefined}>
                      <button
                        type="button"
                        className="agent-avatar-button"
                        onClick={() => void handleAgentCardOpen(agent)}
                        title={agent.displayName ?? agent.name}
                      >
                        <span className={`agent-status-dot ${agent.status === 'offline' ? 'offline' : 'online'}`} />
                      </button>
                      <span className="agent-name">{agent.displayName ?? agent.name}</span>
                      <span className="agent-status-label">{agent.status}</span>
                      {agentCardTarget?.id === agent.id ? (
                        <div className="agent-card" role="dialog" aria-label={agent.name}>
                          <div className="agent-card-header">
                            <span className={`agent-status-dot ${agent.status === 'offline' ? 'offline' : 'online'}`} />
                            <span className="agent-card-name">{agent.displayName ?? agent.name}</span>
                            <span className="agent-card-status">{agent.status}</span>
                          </div>
                          <p className="agent-card-description">
                            {agent.description || copy.agentNoDescription}
                          </p>
                          <div className="agent-card-activity">
                            <p className="agent-card-activity-title">{copy.agentActivity}</p>
                            {agentCardLoading ? (
                              <SpinnerIcon />
                            ) : agentCardActivity.length > 0 ? (
                              <ul className="agent-card-activity-list">
                                {agentCardActivity.map((entry) => (
                                  <li key={entry.id}>
                                    <span className="activity-text">{entry.activity}</span>
                                    <span className="activity-time">{formatRelativeTime(entry.createdAt)}</span>
                                  </li>
                                ))}
                              </ul>
                            ) : (
                              <p className="inline-note">{copy.agentNoActivity}</p>
                            )}
                          </div>
                          <div className="agent-card-actions">
                            {agent.status !== 'offline' ? (
                              <>
                                <button
                                  type="button"
                                  className="agent-card-button danger"
                                  onClick={() => void handleAgentStop(agent)}
                                  disabled={agentCardAction !== null}
                                >
                                  {agentCardAction === 'stop' ? copy.agentStopping : copy.agentStop}
                                </button>
                                <button
                                  type="button"
                                  className="agent-card-button"
                                  onClick={() => void handleAgentRestart(agent)}
                                  disabled={agentCardAction !== null}
                                >
                                  {agentCardAction === 'restart' ? copy.agentStarting : copy.agentRestart}
                                </button>
                              </>
                            ) : (
                              <button
                                type="button"
                                className="agent-card-button accent"
                                onClick={() => void handleAgentStart(agent)}
                                disabled={agentCardAction !== null}
                              >
                                {agentCardAction === 'start' ? copy.agentStarting : copy.agentStart}
                              </button>
                            )}
                          </div>
                        </div>
                      ) : null}
                    </div>
                  ))}
                </div>
              </div>

              <div className="dashboard-panel">
                <p className="eyebrow">{copy.dashboardActiveChannels ?? 'Active Channels'}</p>
                <div className="dashboard-channel-list">
                  {dashboardData.channels
                    .filter((ch) => !ch.isArchived && ch.lastMessageAt)
                    .sort((a, b) => (b.lastMessageAt ?? '').localeCompare(a.lastMessageAt ?? ''))
                    .slice(0, 5)
                    .map((channel) => {
                      const unread = dashboardData.unread.find((u) => u.channelId === channel.id)
                      return (
                        <div key={channel.id} className="dashboard-channel-row">
                          <span className="channel-name">#{channel.name}</span>
                          {unread && unread.unreadCount > 0 ? (
                            <span className="channel-unread-badge">{unread.unreadCount}</span>
                          ) : null}
                        </div>
                      )
                    })}
                </div>
              </div>
            </div>
          </section>
        ) : dashboardLoading ? (
          <div className="dashboard-loading">
            <SpinnerIcon />
          </div>
        ) : dashboardError ? (
          <div className="dashboard-error">
            <p className="inline-note">{dashboardError}</p>
          </div>
        ) : null}

        <div className="launch-dock">
          <button
            className={`launch-button${workspaceLaunching ? ' launching' : ''}`}
            onClick={() => {
              launchButtonAccentRef.current = selectedThemeAccent
              void handleWorkspaceOpen(selectedServiceSlug || undefined)
            }}
            disabled={serviceActionBusy}
          >
            {busyAction === 'workspace' ? copy.launching : stackButtonLabel}
          </button>
        </div>
      </section>

      {messageReminder ? (
        <section
          className="message-reminder-toast"
          role="status"
          aria-live="polite"
          aria-label={copy.messageReminderTitle}
        >
          <button
            type="button"
            className="message-reminder-main"
            onClick={() => void handleMessageReminderOpen(messageReminder)}
            title={`${copy.messageReminderOpen} ${messageReminder.serverName}`}
          >
            <span className="message-reminder-kicker">{copy.messageReminderTitle}</span>
            <span className="message-reminder-title">
              {messageReminder.senderName} · {messageReminder.serverName}
            </span>
            <span className="message-reminder-body">{messageReminder.contentPreview}</span>
          </button>
          <button
            type="button"
            className="message-reminder-close"
            onClick={handleMessageReminderDismiss}
            aria-label={copy.messageReminderDismiss}
            title={copy.messageReminderDismiss}
          >
            ×
          </button>
        </section>
      ) : null}

      {serviceLogViewer ? (
        <section
          className="service-log-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              handleServiceLogClose()
            }
          }}
        >
          <section
            className="service-log-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="service-log-title"
          >
            <header className="service-log-head">
              <div className="service-log-heading">
                <p className="eyebrow">{copy.serverLogTitle}</p>
                <h2 id="service-log-title">{serviceLogViewer.serverName}</h2>
                <code
                  className="service-log-path"
                  title={serviceLogViewer.snapshot?.path ?? serviceLogViewer.serverSlug}
                >
                  {serviceLogViewer.snapshot?.path ?? serviceLogViewer.serverSlug}
                </code>
              </div>
              <button
                className="icon-action-button compact"
                type="button"
                onClick={handleServiceLogClose}
                aria-label={copy.close}
                title={copy.close}
              >
                <XIcon />
              </button>
            </header>

            <div className="service-log-controls">
              <div className="service-log-timebar">
                <ServiceLogTimeField
                  label={copy.serverLogFrom}
                  value={serviceLogViewer.rangeStart}
                  disabled={serviceLogViewer.loading}
                  onChange={(part, value) =>
                    handleServiceLogRangePartChange('rangeStart', part, value)
                  }
                />
                <ServiceLogTimeField
                  label={copy.serverLogTo}
                  value={serviceLogViewer.rangeEnd}
                  disabled={serviceLogViewer.loading}
                  onChange={(part, value) =>
                    handleServiceLogRangePartChange('rangeEnd', part, value)
                  }
                />
                <label className="service-log-range-select">
                  <ClockIcon />
                  <select
                    value={serviceLogViewer.rangePresetMs ?? ''}
                    onChange={(event) => {
                      const durationMs = Number(event.target.value)
                      if (durationMs > 0) {
                        handleServiceLogRangePresetChange(durationMs)
                      }
                    }}
                    disabled={serviceLogViewer.loading}
                    aria-label={copy.serverLogRange}
                    title={copy.serverLogRange}
                  >
                    <option value="">{copy.serverLogCustomRange}</option>
                    {SERVICE_LOG_QUICK_RANGES.map((range) => (
                      <option key={range.key} value={range.durationMs}>
                        {copy[range.key]}
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  className="icon-action-button compact service-log-range-button"
                  type="button"
                  onClick={() =>
                    void handleServiceLogOpenWithRange(
                      serviceLogViewer.serverSlug,
                      serviceLogViewer.rangeStart,
                      serviceLogViewer.rangeEnd,
                    )
                  }
                  disabled={serviceLogViewer.loading}
                  aria-label={copy.serverLogRangeApply}
                  title={copy.serverLogRangeApply}
                >
                  <ServiceActionIcon type="refresh" busy={serviceLogViewer.loading} />
                </button>
              </div>

              <div className="service-log-toolbar">
                <label className="server-search service-log-search">
                  <ServerSearchIcon />
                  <span className="sr-only">{copy.serverLogSearch}</span>
                  <input
                    ref={serviceLogSearchRef}
                    value={serviceLogViewer.query}
                    onChange={(event) => handleServiceLogQueryChange(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') {
                        event.preventDefault()
                        handleServiceLogMatchStep(event.shiftKey ? -1 : 1)
                      }
                    }}
                    placeholder={copy.serverLogSearch}
                    aria-label={copy.serverLogSearch}
                    disabled={!serviceLogViewer.snapshot || serviceLogViewer.loading}
                  />
                </label>
                <span className="status-chip service-log-count">
                  {serviceLogStatusLabel}
                </span>
                <div className="service-log-actions">
                  <button
                    className="icon-action-button compact"
                    type="button"
                    onClick={() => handleServiceLogMatchStep(-1)}
                    disabled={serviceLogSearching || serviceLogMatchCount === 0}
                    aria-label={copy.serverLogPreviousMatch}
                    title={copy.serverLogPreviousMatch}
                  >
                    <ChevronIcon direction="up" />
                  </button>
                  <button
                    className="icon-action-button compact"
                    type="button"
                    onClick={() => handleServiceLogMatchStep(1)}
                    disabled={serviceLogSearching || serviceLogMatchCount === 0}
                    aria-label={copy.serverLogNextMatch}
                    title={copy.serverLogNextMatch}
                  >
                    <ChevronIcon direction="down" />
                  </button>
                </div>
              </div>
            </div>

            {serviceLogViewer.error ? (
              <p className="inline-note error service-log-error" role="alert">
                {serviceLogViewer.error}
              </p>
            ) : null}

            <div className="service-log-body">
              {serviceLogViewer.loading ? (
                <div className="service-loading-row" role="status" aria-live="polite">
                  <SpinnerIcon />
                  <span>{copy.serverLogLoading}</span>
                </div>
              ) : serviceLogViewer.snapshot?.content ? (
                <pre className="service-log-content" ref={serviceLogContentRef} tabIndex={0}>{serviceLogViewer.snapshot.content}</pre>
              ) : (
                <p className="inline-note service-log-empty">{copy.serverLogEmpty}</p>
              )}
            </div>

            {serviceLogViewer.snapshot?.truncated ? (
              <p className="inline-note service-log-truncated">
                {copy.serverLogTruncated}
              </p>
            ) : null}
          </section>
        </section>
      ) : null}
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

function AccountAvatar({
  account,
}: {
  account: ServiceAccountSnapshot | null
}) {
  const initials = account?.initials?.trim().slice(0, 2) || 'S'

  return (
    <span className="account-avatar" aria-hidden="true">
      <span className="account-avatar-fallback">{initials}</span>
      {account?.avatarUrl ? (
        <img
          src={account.avatarUrl}
          alt=""
          referrerPolicy="no-referrer"
          onError={(event) => {
            event.currentTarget.style.display = 'none'
          }}
        />
      ) : null}
    </span>
  )
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
            {deleting ? <SpinnerIcon /> : <XIcon />}
          </button>
        </div>
      ) : null}
    </div>
  )
}

interface ThemeStyleRowProps {
  style: ThemeStyleDefinition
  name: string
  summary: string
  selected: boolean
  busy: boolean
  onSelect: () => void
}

function ThemeStyleRow({
  style,
  name,
  summary,
  selected,
  busy,
  onSelect,
}: ThemeStyleRowProps) {
  return (
    <div
      className={`theme-row theme-style-row${selected ? ' selected' : ''}`}
      role="radio"
      aria-checked={selected}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onSelect()
        }
      }}
      onClick={onSelect}
    >
      <span className="theme-style-preview" aria-hidden="true">
        {style.preview.map((color, index) => (
          <span key={`${style.id}-${index}`} style={{ background: color }} />
        ))}
      </span>
      <span className="theme-row-copy">
        <span className="theme-row-name">{name}</span>
        <span className="theme-row-summary">{summary}</span>
      </span>
      <span className="theme-row-actions visible">
        {busy ? <SpinnerIcon /> : null}
      </span>
    </div>
  )
}

function ServiceLogTimeField({
  label,
  value,
  disabled,
  onChange,
}: {
  label: string
  value: string
  disabled: boolean
  onChange: (part: 'date' | 'time', value: string) => void
}) {
  return (
    <fieldset className="service-log-time-field">
      <legend>{label}</legend>
      <label className="service-log-time-input">
        <CalendarIcon />
        <input
          type="date"
          value={getDatetimeDatePart(value)}
          onChange={(event) => onChange('date', event.target.value)}
          disabled={disabled}
          aria-label={`${label} date`}
        />
      </label>
      <label className="service-log-time-input">
        <ClockIcon />
        <input
          type="time"
          step={1}
          value={getDatetimeTimePart(value)}
          onChange={(event) => onChange('time', event.target.value)}
          disabled={disabled}
          aria-label={`${label} time`}
        />
      </label>
    </fieldset>
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

const LOG_SEARCH_CHUNK_SIZE = 64 * 1024

function getDefaultServiceLogRange() {
  return getServiceLogRangeForDuration(DEFAULT_SERVICE_LOG_RANGE_MS)
}

function getServiceLogRangeForDuration(durationMs: number) {
  const end = new Date()
  const start = new Date(end.getTime() - durationMs)
  return {
    start: toDatetimeLocalValue(start),
    end: toDatetimeLocalValue(end),
  }
}

function toDatetimeLocalValue(date: Date) {
  const pad = (value: number) => String(value).padStart(2, '0')
  return [
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`,
    `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`,
  ].join('T')
}

function datetimeLocalToEpochMs(value: string) {
  const parsed = new Date(value)
  return Number.isFinite(parsed.getTime()) ? parsed.getTime() : null
}

function getDatetimeDatePart(value: string) {
  return value.split('T')[0] ?? ''
}

function getDatetimeTimePart(value: string) {
  const time = value.split('T')[1] ?? ''
  return time.length === 5 ? `${time}:00` : time
}

function updateDatetimeLocalPart(value: string, part: 'date' | 'time', nextValue: string) {
  const currentDate = getDatetimeDatePart(value) || getDatetimeDatePart(toDatetimeLocalValue(new Date()))
  const currentTime = getDatetimeTimePart(value) || '00:00:00'
  return part === 'date'
    ? `${nextValue || currentDate}T${currentTime}`
    : `${currentDate}T${normalizeTimeInput(nextValue || currentTime)}`
}

function normalizeTimeInput(value: string) {
  return value.length === 5 ? `${value}:00` : value
}

function countLogLines(content: string) {
  if (!content) {
    return 0
  }

  let lines = 1
  for (let index = 0; index < content.length; index += 1) {
    const code = content.charCodeAt(index)
    if (code === 10) {
      lines += 1
    } else if (code === 13) {
      lines += 1
      if (content.charCodeAt(index + 1) === 10) {
        index += 1
      }
    }
  }
  return lines
}

function scanLogMatchesInChunks(
  content: string,
  query: string,
  activeMatchIndex: number,
  onComplete: (result: Omit<ServiceLogSearchState, 'searching'>) => void,
  isCancelled: () => boolean,
) {
  const needle = query.trim().toLowerCase()
  if (!needle) {
    onComplete({
      query: '',
      activeMatchIndex: 0,
      count: 0,
      activeStart: -1,
      activeEnd: -1,
    })
    return
  }

  let count = 0
  let scanStart = 0
  let activeStart = -1
  let lastMatchStart = -1
  const needleLength = needle.length

  const scanNextChunk = () => {
    if (isCancelled()) {
      return
    }

    const acceptedEnd = Math.min(content.length, scanStart + LOG_SEARCH_CHUNK_SIZE)
    const chunkEnd = Math.min(content.length, acceptedEnd + needleLength - 1)
    const chunk = content.slice(scanStart, chunkEnd).toLowerCase()
    let cursor = 0

    while (cursor < chunk.length) {
      const next = chunk.indexOf(needle, cursor)
      if (next === -1) {
        break
      }

      const matchStart = scanStart + next
      if (matchStart >= acceptedEnd) {
        break
      }

      if (count === activeMatchIndex) {
        activeStart = matchStart
      }
      lastMatchStart = matchStart
      count += 1
      cursor = next + needleLength
    }

    scanStart += LOG_SEARCH_CHUNK_SIZE
    if (scanStart < content.length) {
      window.setTimeout(scanNextChunk, 0)
      return
    }

    const resolvedStart = activeStart >= 0 ? activeStart : lastMatchStart
    onComplete({
      query,
      activeMatchIndex,
      count,
      activeStart: resolvedStart,
      activeEnd: resolvedStart >= 0 ? resolvedStart + needleLength : -1,
    })
  }

  scanNextChunk()
}

function getLogTextRange(container: HTMLElement | null, start: number, end: number) {
  if (!container || start < 0 || end <= start) {
    return null
  }

  const range = document.createRange()
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT)
  let offset = 0
  let node = walker.nextNode()
  let started = false

  while (node) {
    const textLength = node.textContent?.length ?? 0
    const nextOffset = offset + textLength

    if (!started && start >= offset && start <= nextOffset) {
      range.setStart(node, start - offset)
      started = true
    }
    if (started && end >= offset && end <= nextOffset) {
      range.setEnd(node, end - offset)
      return range
    }

    offset = nextOffset
    node = walker.nextNode()
  }

  return null
}

function applyLogHighlight(name: string, range: Range) {
  clearLogHighlight(name)
  const mark = document.createElement('mark')
  mark.className = 'active'
  mark.dataset.logHighlight = name
  try {
    range.surroundContents(mark)
  } catch {
    const fragment = range.extractContents()
    mark.append(fragment)
    range.insertNode(mark)
  }
}

function clearLogHighlight(name: string) {
  document.querySelectorAll(`mark[data-log-highlight="${name}"]`).forEach((mark) => {
    const parent = mark.parentNode
    mark.replaceWith(document.createTextNode(mark.textContent ?? ''))
    parent?.normalize()
  })
}

function scrollLogRangeIntoView(range: Range, container: HTMLElement | null) {
  if (!container) {
    return
  }

  const target = container.querySelector('mark[data-log-highlight]')
  const targetRect = target?.getBoundingClientRect()
  const rangeRect = targetRect ?? range.getBoundingClientRect()
  if (rangeRect.height === 0 && rangeRect.width === 0) {
    return
  }
  const containerRect = container.getBoundingClientRect()

  const targetTop =
    rangeRect.top - containerRect.top + container.scrollTop - container.clientHeight / 2
  container.scrollTo({
    top: Math.max(0, targetTop),
    behavior: 'smooth',
  })
}

type ServiceActionIconType = 'start' | 'stop' | 'refresh'
type OptionIconType =
  | (typeof THEME_MODES)[number]['icon']
  | (typeof LANGUAGE_OPTIONS)[number]['icon']

function OptionIcon({ type }: { type: OptionIconType }) {
  if (type === 'latin') {
    return (
      <svg
        className="option-icon"
        aria-hidden="true"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.1"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M7 18 12 6l5 12" />
        <path d="M9.2 14h5.6" />
      </svg>
    )
  }

  if (type === 'han') {
    return (
      <svg
        className="option-icon han-icon"
        aria-hidden="true"
        viewBox="0 0 1024 1024"
        fill="currentColor"
      >
        <path d="M555.231787 330.203429v-107.997284h-68.202727v108.038827H263.433935v273.457531H487.02906v210.976899h68.202727V603.70431h224.21827V330.203429H555.231787z m-68.202727 209.074952h-157.337694v-144.605675h157.335888v144.605675z m226.131053 0H555.195662v-144.605675h157.962645v144.605675z" />
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

function XIcon() {
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
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  )
}

function StyleIcon() {
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
      <path d="M12 2.7 2 7l10 5 10-5-10-4.3Z" />
      <path d="m2 17 10 5 10-5" />
      <path d="m2 12 10 5 10-5" />
    </svg>
  )
}

function LogsIcon() {
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
      <rect width="14" height="16" x="4" y="3" rx="2" />
      <path d="M8 8h6" />
      <path d="M8 12h4" />
      <circle cx="16.5" cy="16.5" r="2.5" />
      <path d="m18.5 18.5 2 2" />
    </svg>
  )
}

function CalendarIcon() {
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
      <rect width="16" height="16" x="4" y="5" rx="2" />
      <path d="M8 3v4" />
      <path d="M16 3v4" />
      <path d="M4 10h16" />
    </svg>
  )
}

function ClockIcon() {
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
      <circle cx="12" cy="12" r="8" />
      <path d="M12 8v5" />
      <path d="m12 13 3 2" />
    </svg>
  )
}

function ChevronIcon({ direction }: { direction: 'up' | 'down' }) {
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
      {direction === 'up' ? <path d="m18 15-6-6-6 6" /> : <path d="m6 9 6 6 6-6" />}
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

function getThemeStyleName(style: ThemeStyleDefinition, copy: UiCopy) {
  if (style.id === 'original') {
    return copy.themeStyleOriginalName
  }
  if (style.id === 'default') {
    return copy.themeStyleDefaultName
  }
  return style.name
}

function getThemeStyleSummary(style: ThemeStyleDefinition, copy: UiCopy) {
  if (style.id === 'original') {
    return copy.themeStyleOriginalSummary
  }
  if (style.id === 'default') {
    return copy.themeStyleDefaultSummary
  }
  return style.summary
}

function exportThemeStyleFile(style: ThemeStyleDefinition) {
  const payload = {
    schema: 'slock-desktop.theme-style.v1',
    style: style.config,
  }
  const blob = new Blob([`${JSON.stringify(payload, null, 2)}\n`], {
    type: 'application/json',
  })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `${toFileSlug(style.name || style.id)}.slock-style.json`
  document.body.append(anchor)
  anchor.click()
  anchor.remove()
  URL.revokeObjectURL(url)
}

function readThemeStyleConfig(value: unknown): ThemeStyleConfig {
  if (!isObjectRecord(value)) {
    throw new Error('Invalid style file')
  }
  const candidate = isObjectRecord(value.style)
    ? value.style
    : isObjectRecord(value.config)
      ? value.config
      : value
  if (!isObjectRecord(candidate)) {
    throw new Error('Invalid style file')
  }
  return candidate as unknown as ThemeStyleConfig
}

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function toFileSlug(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'theme-style'
}

function createNewThemeDraft(accent = DEFAULT_NEW_THEME_ACCENT): NewThemeDraft {
  const normalized = normalizeHexColor(accent) ?? DEFAULT_NEW_THEME_ACCENT
  const rgb = hexToRgb(normalized)
  return {
    name: '',
    accent: normalized,
    hexInput: normalized.toUpperCase(),
    rgbInput: {
      r: String(rgb.r),
      g: String(rgb.g),
      b: String(rgb.b),
    },
  }
}

function syncNewThemeDraftAccent(
  draft: NewThemeDraft,
  accent: string,
): NewThemeDraft {
  const normalized = normalizeHexColor(accent) ?? draft.accent
  const rgb = hexToRgb(normalized)
  return {
    ...draft,
    accent: normalized,
    hexInput: normalized.toUpperCase(),
    rgbInput: {
      r: String(rgb.r),
      g: String(rgb.g),
      b: String(rgb.b),
    },
  }
}

function normalizeHexColor(value: string) {
  const compact = value.trim().replace(/^#/, '')
  if (/^[0-9a-fA-F]{3}$/.test(compact)) {
    return `#${compact
      .split('')
      .map((part) => `${part}${part}`)
      .join('')}`.toLowerCase()
  }

  if (/^[0-9a-fA-F]{6}$/.test(compact)) {
    return `#${compact}`.toLowerCase()
  }

  return null
}

function hexToRgb(hex: string) {
  const normalized = normalizeHexColor(hex) ?? DEFAULT_NEW_THEME_ACCENT
  const value = normalized.slice(1)
  return {
    r: parseInt(value.slice(0, 2), 16),
    g: parseInt(value.slice(2, 4), 16),
    b: parseInt(value.slice(4, 6), 16),
  }
}

function rgbToHex(r: number, g: number, b: number) {
  return `#${[r, g, b]
    .map((value) => value.toString(16).padStart(2, '0'))
    .join('')}`
}

function hsvToHex(hue: number, saturation: number, value: number) {
  const chroma = value * saturation
  const huePrime = (((hue % 360) + 360) % 360) / 60
  const x = chroma * (1 - Math.abs((huePrime % 2) - 1))
  const match = value - chroma
  const [r1, g1, b1] =
    huePrime < 1
      ? [chroma, x, 0]
      : huePrime < 2
        ? [x, chroma, 0]
        : huePrime < 3
          ? [0, chroma, x]
          : huePrime < 4
            ? [0, x, chroma]
            : huePrime < 5
              ? [x, 0, chroma]
              : [chroma, 0, x]

  return rgbToHex(
    Math.round((r1 + match) * 255),
    Math.round((g1 + match) * 255),
    Math.round((b1 + match) * 255),
  )
}

function rgbToHsv(r: number, g: number, b: number) {
  const red = r / 255
  const green = g / 255
  const blue = b / 255
  const max = Math.max(red, green, blue)
  const min = Math.min(red, green, blue)
  const delta = max - min
  const saturation = max === 0 ? 0 : delta / max
  let hue = 0

  if (delta !== 0) {
    if (max === red) {
      hue = 60 * (((green - blue) / delta) % 6)
    } else if (max === green) {
      hue = 60 * ((blue - red) / delta + 2)
    } else {
      hue = 60 * ((red - green) / delta + 4)
    }
  }

  return {
    h: (hue + 360) % 360,
    s: saturation,
    v: max,
  }
}

function getAccentFromWheelPointer(clientX: number, clientY: number, target: HTMLElement) {
  const rect = target.getBoundingClientRect()
  const radius = Math.min(rect.width, rect.height) / 2
  const dx = clientX - (rect.left + rect.width / 2)
  const dy = clientY - (rect.top + rect.height / 2)
  const distance = Math.min(radius, Math.hypot(dx, dy))
  const saturation = radius === 0 ? 0 : distance / radius
  const hue = (Math.atan2(dy, dx) * 180) / Math.PI + 180
  return hsvToHex(hue, saturation, 0.96)
}

function getAccentWheelMarkerStyle(accent: string): CSSProperties {
  const rgb = hexToRgb(accent)
  const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b)
  const angle = (hsv.h - 180) * (Math.PI / 180)
  const radius = Math.max(0.08, Math.min(1, hsv.s)) * 46
  const x = 50 + Math.cos(angle) * radius
  const y = 50 + Math.sin(angle) * radius

  return {
    '--wheel-x': `${x}%`,
    '--wheel-y': `${y}%`,
    '--custom-accent': accent,
  } as CSSProperties
}

function sanitizeRgbInput(value: string) {
  return value.replace(/\D/g, '').slice(0, 3)
}

function parseRgbInput(input: NewThemeDraft['rgbInput']) {
  const r = Number(input.r)
  const g = Number(input.g)
  const b = Number(input.b)
  if (
    !input.r ||
    !input.g ||
    !input.b ||
    [r, g, b].some((value) => !Number.isInteger(value) || value < 0 || value > 255)
  ) {
    return null
  }

  return { r, g, b }
}

function getAccountEmailLabel(
  account: ServiceAccountSnapshot | null,
  copy: UiCopy,
) {
  return account?.email?.trim() || account?.displayName?.trim() || copy.accountEmailUnavailable
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
  selectedServerSlug = service.selectedServerSlug,
) {
  if (isServiceServerRunning(service, server.slug)) {
    return copy.serviceRunning
  }

  if (server.slug === selectedServerSlug || server.selected) {
    return service.configured ? copy.serviceIdle : copy.serviceNotLinked
  }

  return getMachineStatusLabel(server.machineStatus, copy)
}

function isSelectedServiceRunning(
  service: BootstrapPayload['service'],
  selectedServerSlug: string,
) {
  return isServiceServerRunning(service, selectedServerSlug)
}

function isServiceServerRunning(
  service: BootstrapPayload['service'],
  serverSlug: string,
) {
  const activeServerSlug = service.activeServerSlug.trim()
  const selectedServerSlug = serverSlug.trim()
  return Boolean(
    service.running &&
      activeServerSlug &&
      selectedServerSlug &&
      activeServerSlug === selectedServerSlug,
  )
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

function formatRelativeTime(isoString: string): string {
  const now = Date.now()
  const then = new Date(isoString).getTime()
  const diffMs = now - then
  const diffSec = Math.floor(diffMs / 1000)
  const diffMin = Math.floor(diffSec / 60)
  const diffHour = Math.floor(diffMin / 60)
  const diffDay = Math.floor(diffHour / 24)

  if (diffSec < 60) { return '<1m' }
  if (diffMin < 60) { return `${diffMin}m` }
  if (diffHour < 24) { return `${diffHour}h` }
  return `${diffDay}d`
}

function waitForNextPaint() {
  return new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
  })
}

export default App
