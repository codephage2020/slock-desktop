import {
  type CSSProperties,
  startTransition,
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { listen } from '@tauri-apps/api/event'
import { io, type Socket } from 'socket.io-client'
import './App.css'
import './Settings.css'
import {
  activateAccount,
  type AgentListItem,
  type AgentTemplate,
  type BootstrapPayload,
  createAgent,
  type DesktopUpdateCheck,
  deleteAgentTemplate,
  fetchAgentDetail,
  fetchAgents,
  fetchMachines,
  type InboxFeedItem,
  type InboxMessage,
  type MachineListItem,
  type MessageAttachment,
  type ServiceAccountSnapshot,
  type ServiceLogSnapshot,
  type ThemeDefinition,
  type ThemeStyleDefinition,
  checkDesktopUpdate,
  checkServerMachines,
  createCustomTheme,
  deleteCustomTheme,
  fetchChannelMessages,
  fetchDashboard,
  fetchDmChannels,
  fetchInbox,
  fetchThreadMessages,
  getAgentTemplates,
  getSocketAuth,
  installDesktopUpdate,
  forgetAccount,
  loadBootstrap,
  markChannelRead,
  openComputerCreatePage,
  openLogin,
  openServiceLog,
  openWorkspace,
  prepareDaemonCommand,
  refreshServiceServerCatalog,
  refreshServiceServerStatus,
  saveAgentTemplate,
  selectServiceServer,
  sendMessage,
  startService,
  stopService,
  switchAccount,
  updateCustomThemeAccent,
  updateLanguage,
  updateTheme,
  updateThemeMode,
  updateThemeStyle,
  uploadAttachment,
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
const MESSAGE_REMINDER_MAX_VISIBLE = 3

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
    agentCreate: 'Agent',
    agentNewAgent: 'New Agent',
    agentTemplates: 'Templates',
    agentBack: '← Back',
    agentCreateTitle: 'Create Agent',
    agentCreateMode: 'Start from',
    agentModeBlank: 'Scratch',
    agentModeTemplate: 'Template',
    agentModeAgent: 'Agent',
    agentName: 'Name',
    agentDisplayName: 'Display Name',
    agentComputer: 'Computer',
    agentInstructions: 'Instructions',
    agentAdvanced: 'Advanced',
    agentModel: 'Model',
    agentMaxTurns: 'Max Turns',
    agentChannel: 'Channel',
    agentSaveTemplate: 'Save as Template',
    agentCancelBtn: 'Cancel',
    agentCreateBtn: 'Create',
    agentCreating: 'Creating…',
    agentSelectTemplate: 'Select template…',
    agentSelectAgent: 'Select agent…',
    agentSelectComputer: 'Select computer…',
    agentNoComputers: 'No computers available',
    agentNoTemplates: 'No templates yet',
    agentNoAgents: 'No agents found',
    agentTemplateTitle: 'Templates',
    agentNewTemplate: '+ New Template',
    agentFromAgent: '+ From Existing Agent',
    agentEditTemplate: 'Edit Template',
    agentTemplateName: 'Template Name',
    agentTemplateSave: 'Save',
    agentTemplateDelete: 'Delete',
    agentTemplateDeleting: 'Deleting…',
    agentTemplateTurns: 'turns',
    agentTemplateUntitled: 'Untitled',
    agentEnvVars: 'Environment Variables',
    agentEnvVarsKey: 'Key',
    agentEnvVarsValue: 'Value',
    agentEnvVarsAdd: '+ Add Variable',
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
    noComputerTitle: 'No Computer',
    noComputerMessage: 'This server does not have a computer yet. Create one to start the daemon.',
    noComputerCreate: 'Create Computer',
    noComputerWaiting: 'After creating the computer in your browser, click Refresh to continue.',
    noComputerRefresh: 'Refresh',
    noComputerChecking: 'Checking…',
    noComputerReady: 'Computer detected! You can now start the daemon.',
    noComputerStart: 'Start Daemon',
    noComputerCopy: 'Copy',
    noComputerExecute: 'Execute',
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
    themeStyleOriginalName: 'Original style',
    themeStyleOriginalSummary: 'Current web UI without desktop overrides.',
    themeStyleDefaultName: 'Default style',
    themeStyleDefaultSummary: 'Desktop refined style.',
    themeImportInvalid: 'Invalid style file.',
    themeNewLabel: 'New theme',
    themeDelete: 'Delete',
    themeEmptyHint: 'No custom themes yet — tap + to create one.',
    themeNewTitle: 'Create theme',
    themeCreate: 'Create',
    themeCancel: 'Cancel',
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
    dashboardMyTasks: 'My Tasks',
    dashboardNoMyTasks: 'No tasks assigned to you',
    dashboardRecentTasks: 'Recent Tasks',
    dashboardNoTasks: 'No tasks yet',
    dashboardOpenWorkspace: 'Open in Workspace',
    dashboardNoChannels: 'No active channels — open the workspace to get started',
    dashboardNoAgents: 'No agents configured yet',
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
    inbox: 'Inbox',
    inboxUnread: 'Unread',
    inboxAll: 'All',
    inboxMentions: 'Mentions',
    inboxDMs: 'DMs',
    inboxSearch: 'Search…',
    inboxEmpty: 'No messages yet',
    inboxNoUnread: 'No recent messages',
    inboxSelectThread: 'Select a conversation to view messages',
    inboxSend: 'Send',
    inboxReplyPlaceholder: 'Type a message…',
    inboxSending: 'Sending…',
    inboxThread: 'Thread',
    inboxConversation: 'Conversation',
    inboxUnreadLabel: 'unread',
    inboxUnknownSender: 'Unknown',
    inboxQuickSend: 'Quick Send',
    inboxSelectTarget: 'Select a channel…',
    inboxComposePlaceholder: 'Write a message…',
    inboxBack: 'Back',
    inboxExpandMore: 'Show more',
    inboxCollapse: 'Show less',
    inboxReplies: 'replies',
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
    agentCreate: 'Agent',
    agentNewAgent: '新建 Agent',
    agentTemplates: '模板管理',
    agentBack: '← 返回',
    agentCreateTitle: '创建 Agent',
    agentCreateMode: '创建方式',
    agentModeBlank: '空白',
    agentModeTemplate: '模板',
    agentModeAgent: 'Agent',
    agentName: '名称',
    agentDisplayName: '显示名称',
    agentComputer: '计算机',
    agentInstructions: '指令',
    agentAdvanced: '高级选项',
    agentModel: '模型',
    agentMaxTurns: '最大轮次',
    agentChannel: '频道',
    agentSaveTemplate: '另存为模板',
    agentCancelBtn: '取消',
    agentCreateBtn: '创建',
    agentCreating: '创建中…',
    agentSelectTemplate: '选择模板…',
    agentSelectAgent: '选择 Agent…',
    agentSelectComputer: '选择计算机…',
    agentNoComputers: '暂无可用计算机',
    agentNoTemplates: '暂无模板',
    agentNoAgents: '未找到 Agent',
    agentTemplateTitle: '模板管理',
    agentNewTemplate: '+ 新建模板',
    agentFromAgent: '+ 从现有 Agent 导入',
    agentEditTemplate: '编辑模板',
    agentTemplateName: '模板名称',
    agentTemplateSave: '保存',
    agentTemplateDelete: '删除',
    agentTemplateDeleting: '删除中…',
    agentTemplateTurns: '轮',
    agentTemplateUntitled: '未命名',
    agentEnvVars: '环境变量',
    agentEnvVarsKey: '键',
    agentEnvVarsValue: '值',
    agentEnvVarsAdd: '+ 添加变量',
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
    noComputerTitle: '无 Computer',
    noComputerMessage: '此 server 还没有 computer，需要创建一个才能启动 daemon。',
    noComputerCreate: '创建 Computer',
    noComputerWaiting: '在浏览器中创建 computer 后，点击"刷新"继续。',
    noComputerRefresh: '刷新',
    noComputerChecking: '检查中…',
    noComputerReady: '已检测到 computer，现在可以启动 daemon。',
    noComputerStart: '启动 Daemon',
    noComputerCopy: '复制',
    noComputerExecute: '执行',
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
    themeStyleOriginalName: '原样式',
    themeStyleOriginalSummary: '保留当前 Web UI 原始样式。',
    themeStyleDefaultName: '默认样式',
    themeStyleDefaultSummary: 'Desktop 整理后的样式。',
    themeImportInvalid: '样式文件无效。',
    themeNewLabel: '新建主题',
    themeDelete: '删除',
    themeEmptyHint: '还没有自定义主题，点击 + 新建。',
    themeNewTitle: '新建主题',
    themeCreate: '创建',
    themeCancel: '取消',
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
    dashboardMyTasks: '我的任务',
    dashboardNoMyTasks: '暂无分配给你的任务',
    dashboardRecentTasks: '近期任务',
    dashboardNoTasks: '暂无任务',
    dashboardOpenWorkspace: '在工作区打开',
    dashboardNoChannels: '暂无活跃频道 — 打开工作区开始使用',
    dashboardNoAgents: '暂未配置 Agent',
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
    inbox: '收件箱',
    inboxUnread: '未读',
    inboxAll: '全部',
    inboxMentions: '提及',
    inboxDMs: '私信',
    inboxSearch: '搜索…',
    inboxEmpty: '暂无消息',
    inboxNoUnread: '暂无近期消息',
    inboxSelectThread: '选择一个会话查看消息',
    inboxSend: '发送',
    inboxReplyPlaceholder: '输入消息…',
    inboxSending: '发送中…',
    inboxThread: '话题',
    inboxConversation: '会话',
    inboxUnreadLabel: '条未读',
    inboxUnknownSender: '未知',
    inboxQuickSend: '快速发送',
    inboxSelectTarget: '选择频道…',
    inboxComposePlaceholder: '输入消息内容…',
    inboxBack: '返回',
    inboxExpandMore: '展开更多',
    inboxCollapse: '收起',
    inboxReplies: '条回复',
  },
} as const

type UiCopy = (typeof COPY)[keyof typeof COPY]
type ServiceRefreshPhase = 'catalog' | 'status' | null

interface NewThemeDraft {
  name: string
  accent: string
}


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
  const [newThemeDraft, setNewThemeDraft] = useState<NewThemeDraft | null>(null)
  const [accountMenuOpen, setAccountMenuOpen] = useState(false)
  const [serverPanelOpen, setServerPanelOpen] = useState(false)
  const [stylePanelOpen, setStylePanelOpen] = useState(false)
  const [agentPanelOpen, setAgentPanelOpen] = useState(false)
  const [agentPanelView, setAgentPanelView] = useState<'menu' | 'create' | 'templates' | 'template-edit'>('menu')
  const [agentCreateMode, setAgentCreateMode] = useState<'scratch' | 'template' | 'agent'>('scratch')
  const [agentCreateForm, setAgentCreateForm] = useState({
    name: '',
    displayName: '',
    machineId: '',
    instructions: '',
    model: '',
    maxTurns: 0,
    channelId: '',
    templateId: '',
    sourceAgentId: '',
    envVars: [] as { key: string; value: string }[],
  })
  const [agentCreateAdvanced, setAgentCreateAdvanced] = useState(false)
  const [agentCreateBusy, setAgentCreateBusy] = useState(false)
  const [agentMachines, setAgentMachines] = useState<MachineListItem[]>([])
  const [agentList, setAgentList] = useState<AgentListItem[]>([])
  const [agentTemplateList, setAgentTemplateList] = useState<AgentTemplate[]>([])
  const [agentEditTemplate, setAgentEditTemplate] = useState<AgentTemplate | null>(null)
  const [agentTemplateBusy, setAgentTemplateBusy] = useState(false)
  const [releaseNotesOpen, setReleaseNotesOpen] = useState(false)
  const [computerCreateFlow, setComputerCreateFlow] = useState<{
    phase: 'prompt' | 'waiting' | 'ready' | 'command'
    serverSlug: string
    createUrl: string
    checking: boolean
    existingMachineIds: string[]
    machineId?: string
    machineName?: string
    daemonCommand?: string
    displayCommand?: string
  } | null>(null)
  const accountMenuRef = useRef<HTMLDivElement | null>(null)
  const serverPanelRef = useRef<HTMLDivElement | null>(null)
  const agentPanelRef = useRef<HTMLDivElement | null>(null)
  const stylePanelRef = useRef<HTMLDivElement | null>(null)
  const releaseNotesRef = useRef<HTMLDivElement | null>(null)
  const newNameInputRef = useRef<HTMLInputElement | null>(null)
  const serviceLogSearchRef = useRef<HTMLInputElement | null>(null)
  const serviceLogContentRef = useRef<HTMLPreElement | null>(null)
  // Unified inbox state (multi-server)
  type UnifiedItem = {
    id: string
    serverSlug: string
    serverName: string
    channelId: string
    channelName: string
    type: 'channel' | 'thread' | 'dm'
    unreadCount: number
    lastMessageAt: string | null
    displayName: string | null
    parentChannelName: string | null
    parentChannelId: string | null
    parentMessageId: string | null
    avatarUrl: string | null
    // Preview from latest message
    lastSenderName: string | null
    lastPreview: string | null
    latestMessageId: string | null
    replyCount: number | null
  }
  type ServerChannelGroup = {
    serverSlug: string
    serverName: string
    channels: { id: string; name: string; type: string; unreadCount: number }[]
  }
  const [unifiedItems, setUnifiedItems] = useState<UnifiedItem[]>([])
  const [serverChannelGroups, setServerChannelGroups] = useState<ServerChannelGroup[]>([])
  const [inboxLoading, setInboxLoading] = useState(false)
  const [inboxLoadingMore, setInboxLoadingMore] = useState(false)
  const [inboxHasMore, setInboxHasMore] = useState(false)
  const [inboxOffset, setInboxOffset] = useState(0)
  const inboxHasLoadedRef = useRef(false)
  const [inboxFilter, setInboxFilter] = useState<'all' | 'unread'>('all')
  const [inboxSearch, setInboxSearch] = useState('')
  const [selectedChannel, setSelectedChannel] = useState<{ serverSlug: string; channelId: string; itemType?: 'channel' | 'thread' | 'dm' } | null>(null)
  const [expandedServers, setExpandedServers] = useState<Set<string>>(new Set())
  // Quick send state
  const [selectedQuickServer, setSelectedQuickServer] = useState<string | null>(null)
  const [quickSendTarget, setQuickSendTarget] = useState<{ serverSlug: string; channelId: string; label: string } | null>(null)
  const [quickSendText, setQuickSendText] = useState('')
  const [quickSendSending, setQuickSendSending] = useState(false)
  const [quickSendServerOpen, setQuickSendServerOpen] = useState(false)
  const [quickSendTargetOpen, setQuickSendTargetOpen] = useState(false)

  // Message detail panel state (#67)
  const [detailMessages, setDetailMessages] = useState<InboxMessage[]>([])
  const [detailLoading, setDetailLoading] = useState(false)
  const [detailHasMore, setDetailHasMore] = useState(false)
  const [replyText, setReplyText] = useState('')
  const [replySending, setReplySending] = useState(false)
  const [replyAttachments, setReplyAttachments] = useState<{ file: File; uploading: boolean; error?: boolean; id?: string }[]>([])
  const detailScrollRef = useRef<HTMLDivElement>(null)
  const detailAutoScrollRef = useRef(true)

  // Refs for stable socket handler access (avoid socket effect dep churn)
  const selectedChannelRef = useRef(selectedChannel)
  selectedChannelRef.current = selectedChannel
  const inboxFilterRef = useRef(inboxFilter)
  inboxFilterRef.current = inboxFilter
  const serversRef = useRef(snapshot?.service.servers ?? [])
  serversRef.current = snapshot?.service.servers ?? []

  // Quick Send attachment state (#68)
  const [quickSendAttachments, setQuickSendAttachments] = useState<{ file: File; uploading: boolean; error?: boolean; id?: string }[]>([])

  const [messageReminders, setMessageReminders] = useState<MessageReminderToast[]>([])
  const messageRemindersRef = useRef<MessageReminderToast[]>([])

  // DM channels for Quick Send target list
  const [serverDmGroups, setServerDmGroups] = useState<{ serverSlug: string; serverName: string; dms: { id: string; name: string; displayName: string | null }[] }[]>([])
  const messageReminderTimersRef = useRef<Map<string, number>>(new Map())
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
  const copyRef = useRef(copy)
  copyRef.current = copy
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
      return
    }

    const closeDraftOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (stylePanelRef.current?.contains(target)) {
        return
      }
      setNewThemeDraft(null)
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
    if (!agentPanelOpen) {
      return
    }

    const closeAgentPanelOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (agentPanelRef.current?.contains(target)) {
        return
      }
      setAgentPanelOpen(false)
      setAgentPanelView('menu')
    }

    document.addEventListener('pointerdown', closeAgentPanelOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeAgentPanelOnOutsidePointer)
  }, [agentPanelOpen])

  useEffect(() => {
    if (!releaseNotesOpen) {
      return
    }

    const closeReleaseNotesOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) {
        return
      }
      if (releaseNotesRef.current?.contains(target)) {
        return
      }
      setReleaseNotesOpen(false)
    }

    document.addEventListener('pointerdown', closeReleaseNotesOnOutsidePointer)
    return () => document.removeEventListener('pointerdown', closeReleaseNotesOnOutsidePointer)
  }, [releaseNotesOpen])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    const timers = messageReminderTimersRef.current

    function clearReminderTimer(id: string) {
      const existing = timers.get(id)
      if (existing !== undefined) {
        window.clearTimeout(existing)
        timers.delete(id)
      }
    }

    function removeReminder(id: string) {
      clearReminderTimer(id)
      const current = messageRemindersRef.current
      const next = current.filter((r) => r.id !== id)
      messageRemindersRef.current = next
      setMessageReminders(next)
    }

    void listen<MessageReminderToast>('slock-message-reminder', (event) => {
      const reminder = event.payload
      const current = messageRemindersRef.current

      // Deduplicate — skip if already showing
      if (current.some((r) => r.id === reminder.id)) return

      // Overflow — evict oldest and clean its timer
      if (current.length >= MESSAGE_REMINDER_MAX_VISIBLE) {
        clearReminderTimer(current[0].id)
        current.shift()
      }

      // Enqueue
      current.push(reminder)
      const next = [...current]
      messageRemindersRef.current = next
      setMessageReminders(next)

      // Create auto-dismiss timer
      const timerId = window.setTimeout(() => {
        removeReminder(reminder.id)
      }, MESSAGE_REMINDER_TOAST_MS)
      timers.set(reminder.id, timerId)
    }).then((cleanup) => {
      unlisten = cleanup
    })

    return () => {
      unlisten?.()
      for (const timerId of timers.values()) {
        window.clearTimeout(timerId)
      }
      timers.clear()
    }
  }, [])

  // Map a single InboxFeedItem to UnifiedItem
  const mapInboxFeedItem = useCallback((item: InboxFeedItem, serverSlug: string, serverName: string, threadLabel: string): UnifiedItem | null => {
    const kind = item.kind as 'channel' | 'thread' | 'dm'
    const channelId = kind === 'thread'
      ? (item.threadChannelId ?? item.channelId ?? '')
      : (item.channelId ?? '')
    if (!channelId) return null

    let channelName: string
    if (kind === 'channel') {
      channelName = item.channelName ? `#${item.channelName}` : ''
    } else if (kind === 'thread') {
      channelName = item.parentChannelName ? `#${item.parentChannelName}` : threadLabel
    } else {
      channelName = item.channelName ?? ''
    }

    return {
      id: `${serverSlug}:${channelId}`,
      serverSlug,
      serverName,
      channelId,
      channelName,
      type: kind,
      unreadCount: item.unreadCount,
      lastMessageAt: item.lastActivityAt ?? item.lastMessageAt,
      displayName: kind === 'dm' ? item.channelName : null,
      parentChannelName: item.parentChannelName ?? null,
      parentChannelId: item.parentChannelId ?? null,
      parentMessageId: item.parentMessageId ?? null,
      avatarUrl: null,
      lastSenderName: item.lastMessageSenderName ?? null,
      lastPreview: item.latestActivityPreview ?? item.lastMessagePreview ?? null,
      replyCount: item.replyCount ?? null,
      latestMessageId: item.latestActivityMessageId ?? item.lastMessageId ?? null,
    }
  }, [])

  // Load more items (next page) for the current filter
  async function handleLoadMore() {
    if (!snapshot?.service.authenticated || inboxLoadingMore || !inboxHasMore) return
    const servers = snapshot.service.servers.filter((s) => s.apiKeyReady)
    if (servers.length === 0) return

    const apiFilter = inboxFilter === 'unread' ? 'unread' : 'all'

    setInboxLoadingMore(true)
    try {
      const serverResults = await Promise.allSettled(
        servers.map(async (server) => {
          const resp = await fetchInbox(server.slug, { filter: apiFilter, limit: 30, offset: inboxOffset })
          return { server, resp }
        })
      )

      const newItems: UnifiedItem[] = []
      let anyHasMore = false

      for (const r of serverResults) {
        if (r.status !== 'fulfilled') continue
        const { server, resp } = r.value
        if (resp.hasMore) anyHasMore = true

        for (const item of resp.items) {
          const mapped = mapInboxFeedItem(item, server.slug, server.name, copy.inboxThread)
          if (mapped) newItems.push(mapped)
        }
      }

      // Deduplicate by id and append
      setUnifiedItems((prev) => {
        const existingIds = new Set(prev.map((i) => i.id))
        const unique = newItems.filter((i) => !existingIds.has(i.id))
        return [...prev, ...unique]
      })
      setInboxHasMore(anyHasMore)
      setInboxOffset((prev) => prev + 30)
    } finally {
      setInboxLoadingMore(false)
    }
  }

  // Fetch unified inbox data from all servers
  useEffect(() => {
    if (!snapshot?.service.authenticated || !initialServiceRefreshDone) {
      setUnifiedItems([])
      setServerChannelGroups([])
      return
    }

    const servers = snapshot.service.servers.filter((s) => s.apiKeyReady)
    if (servers.length === 0) {
      setUnifiedItems([])
      setServerChannelGroups([])
      return
    }

    let cancelled = false

    // Map filter tab to inbox API filter param (web only supports all/unread)
    const apiFilter = inboxFilter === 'unread' ? 'unread' : 'all'

    async function loadUnifiedInbox() {
      // Show skeleton only on first load; on subsequent refreshes keep old data visible
      if (!inboxHasLoadedRef.current) setInboxLoading(true)
      setInboxOffset(0)
      setInboxHasMore(false)
      try {
        // Fetch inbox feed per server using the unified /channels/inbox API
        const serverResults = await Promise.allSettled(
          servers.map(async (server) => {
            const resp = await fetchInbox(server.slug, { filter: apiFilter, limit: 30, offset: 0 })
            return { server, resp }
          })
        )

        if (!cancelled) {
          const allItems: UnifiedItem[] = []
          let anyHasMore = false

          for (const r of serverResults) {
            if (r.status !== 'fulfilled') continue
            const { server, resp } = r.value
            if (resp.hasMore) anyHasMore = true

            for (const item of resp.items) {
              const mapped = mapInboxFeedItem(item, server.slug, server.name, copy.inboxThread)
              if (mapped) allItems.push(mapped)
            }
          }

          setUnifiedItems(allItems)
          setInboxHasMore(anyHasMore)
          setInboxOffset(30)
          inboxHasLoadedRef.current = true
        }
      } finally {
        if (!cancelled) {
          setInboxLoading(false)
        }
      }
    }

    void loadUnifiedInbox()
    return () => { cancelled = true }
  }, [snapshot?.service.servers, snapshot?.service.authenticated, initialServiceRefreshDone, inboxFilter, copy.inboxThread, mapInboxFeedItem])

  // Stable server slugs key for socket effect (avoid reconnect on unrelated snapshot changes)
  const apiReadySlugs = useMemo(
    () => (snapshot?.service.servers ?? []).filter((s) => s.apiKeyReady).map((s) => s.slug).sort().join(','),
    [snapshot?.service.servers],
  )

  // Socket.IO real-time feed updates (#64)
  useEffect(() => {
    if (!snapshot?.service.authenticated || !initialServiceRefreshDone || !apiReadySlugs) return

    let socket: Socket | null = null
    let cancelled = false
    // Debounce timer for high-frequency events — coalesce into a single re-fetch
    let debounceTimer: ReturnType<typeof setTimeout> | null = null
    const DEBOUNCE_MS = 800

    function scheduleRefresh() {
      if (debounceTimer !== null) clearTimeout(debounceTimer)
      debounceTimer = setTimeout(() => {
        if (cancelled) return
        // Read current values from refs to avoid stale closures
        const currentFilter = inboxFilterRef.current
        const currentServers = serversRef.current.filter((s) => s.apiKeyReady)
        const currentCopy = copyRef.current
        const apiFilter = currentFilter === 'unread' ? 'unread' : 'all'

        Promise.allSettled(
          currentServers.map(async (server) => {
            const resp = await fetchInbox(server.slug, { filter: apiFilter, limit: 30, offset: 0 })
            return { server, resp }
          })
        ).then((serverResults) => {
          if (cancelled) return
          const allItems: UnifiedItem[] = []
          let anyHasMore = false
          for (const r of serverResults) {
            if (r.status !== 'fulfilled') continue
            const { server, resp } = r.value
            if (resp.hasMore) anyHasMore = true
            for (const item of resp.items) {
              const mapped = mapInboxFeedItem(item, server.slug, server.name, currentCopy.inboxThread)
              if (mapped) allItems.push(mapped)
            }
          }
          setUnifiedItems(allItems)
          setInboxHasMore(anyHasMore)
          setInboxOffset(30)
        })

        // Also refresh detail messages if a conversation is open (#70)
        const sel = selectedChannelRef.current
        if (sel) {
          const fetcher = sel.itemType === 'thread' ? fetchThreadMessages : fetchChannelMessages
          fetcher(sel.serverSlug, sel.channelId, { limit: 50 }).then((resp) => {
            if (cancelled) return
            setDetailMessages(normalizeMessages(resp.messages))
            setDetailHasMore(resp.hasMore)
            // Auto-scroll if user was at bottom
            if (detailAutoScrollRef.current) {
              requestAnimationFrame(() => {
                if (detailScrollRef.current) {
                  detailScrollRef.current.scrollTop = detailScrollRef.current.scrollHeight
                }
              })
            }
          }).catch(() => { /* ignore */ })
        }
      }, DEBOUNCE_MS)
    }

    async function connectSocket() {
      try {
        const auth = await getSocketAuth()
        if (cancelled) return

        socket = io(auth.serverUrl, {
          auth: { token: auth.accessToken },
          transports: ['websocket'],
          reconnection: true,
          reconnectionDelay: 1000,
          reconnectionDelayMax: 5000,
        })

        socket.on('connect', () => {
          console.log('[socket.io] connected')
          // Refresh on connect/reconnect to catch messages missed while offline
          scheduleRefresh()
        })

        socket.on('disconnect', (reason) => {
          console.log('[socket.io] disconnected:', reason)
        })

        socket.on('connect_error', (err) => {
          console.warn('[socket.io] connect error:', err.message)
        })

        // Listen for real-time feed events
        socket.on('message:new', () => {
          scheduleRefresh()
        })

        socket.on('message:updated', () => {
          scheduleRefresh()
        })

        socket.on('message:deleted', () => {
          scheduleRefresh()
        })
      } catch (err) {
        console.warn('[socket.io] failed to get auth:', err)
      }
    }

    void connectSocket()

    return () => {
      cancelled = true
      if (debounceTimer !== null) clearTimeout(debounceTimer)
      if (socket) {
        socket.disconnect()
        socket = null
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- stabilized: only reconnect on auth/server slug changes
  }, [apiReadySlugs, snapshot?.service.authenticated, initialServiceRefreshDone])

  // Fetch joined channels for Quick Send target list (separate from feed)
  useEffect(() => {
    if (!snapshot?.service.authenticated || !initialServiceRefreshDone) {
      setServerChannelGroups([])
      return
    }

    const servers = snapshot.service.servers.filter((s) => s.apiKeyReady)
    if (servers.length === 0) {
      setServerChannelGroups([])
      return
    }

    let cancelled = false

    async function loadChannelGroups() {
      const results = await Promise.allSettled(
        servers.map(async (server) => {
          const dash = await fetchDashboard(server.slug)
          const channels = dash.channels
            .filter((ch) => ch.joined && !ch.isArchived)
            .map((ch) => ({
              id: ch.id,
              name: ch.name,
              type: ch.type,
              unreadCount: 0,
            }))
          return { serverSlug: server.slug, serverName: server.name, channels }
        })
      )

      if (!cancelled) {
        const groups = results
          .filter((r): r is PromiseFulfilledResult<ServerChannelGroup> => r.status === 'fulfilled')
          .map((r) => r.value)
        setServerChannelGroups(groups)
      }
    }

    void loadChannelGroups()
    return () => { cancelled = true }
  }, [snapshot?.service.servers, snapshot?.service.authenticated, initialServiceRefreshDone])

  // Mark as read when a conversation is selected (for unread badge cleanup)
  useEffect(() => {
    if (!selectedChannel) return
    const { serverSlug, channelId } = selectedChannel
    markChannelRead(serverSlug, channelId).then(() => {
      setUnifiedItems((prev) =>
        prev.map((i) =>
          i.serverSlug === serverSlug && i.channelId === channelId
            ? { ...i, unreadCount: 0 }
            : i,
        ),
      )
    }).catch(() => { /* ignore */ })
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: use primitive fields
  }, [selectedChannel?.serverSlug, selectedChannel?.channelId])

  // Fetch messages when a conversation is selected (#67)
  useEffect(() => {
    if (!selectedChannel) {
      setDetailMessages([])
      setDetailHasMore(false)
      setReplyText('')
      setReplyAttachments([])
      return
    }
    const { serverSlug, channelId, itemType } = selectedChannel
    let cancelled = false
    setDetailLoading(true)
    detailAutoScrollRef.current = true

    const fetcher = itemType === 'thread' ? fetchThreadMessages : fetchChannelMessages
    fetcher(serverSlug, channelId, { limit: 50 })
      .then((resp) => {
        if (cancelled) return
        setDetailMessages(normalizeMessages(resp.messages))
        setDetailHasMore(resp.hasMore)
        // Scroll to bottom after initial load
        requestAnimationFrame(() => {
          if (detailScrollRef.current) {
            detailScrollRef.current.scrollTop = detailScrollRef.current.scrollHeight
          }
        })
      })
      .catch(() => {
        if (!cancelled) setDetailMessages([])
      })
      .finally(() => {
        if (!cancelled) setDetailLoading(false)
      })

    return () => { cancelled = true }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional
  }, [selectedChannel?.serverSlug, selectedChannel?.channelId, selectedChannel?.itemType])

  // Fetch DM channels for Quick Send target list (#68)
  useEffect(() => {
    if (!snapshot?.service.authenticated || !initialServiceRefreshDone) {
      setServerDmGroups([])
      return
    }
    const servers = snapshot.service.servers.filter((s) => s.apiKeyReady)
    if (servers.length === 0) {
      setServerDmGroups([])
      return
    }
    let cancelled = false
    Promise.allSettled(
      servers.map(async (server) => {
        const dms = await fetchDmChannels(server.slug)
        return {
          serverSlug: server.slug,
          serverName: server.name,
          dms: dms.map((dm) => ({ id: dm.id, name: dm.name, displayName: dm.displayName })),
        }
      })
    ).then((results) => {
      if (cancelled) return
      const groups = results
        .filter((r): r is PromiseFulfilledResult<{ serverSlug: string; serverName: string; dms: { id: string; name: string; displayName: string | null }[] }> => r.status === 'fulfilled')
        .map((r) => r.value)
      setServerDmGroups(groups)
    })
    return () => { cancelled = true }
  }, [snapshot?.service.servers, snapshot?.service.authenticated, initialServiceRefreshDone])

  // Auto-select server for Quick Send when only one server exists
  useEffect(() => {
    const allSlugs = new Set([
      ...serverChannelGroups.map((g) => g.serverSlug),
      ...serverDmGroups.map((g) => g.serverSlug),
    ])
    if (allSlugs.size === 1) {
      const slug = [...allSlugs][0]
      setSelectedQuickServer(slug)
    } else if (selectedQuickServer && !allSlugs.has(selectedQuickServer)) {
      // Current selected server no longer available
      setSelectedQuickServer(null)
      setQuickSendTarget(null)
      setQuickSendText('')
      setQuickSendAttachments([])
    }
  }, [serverChannelGroups, serverDmGroups, selectedQuickServer])

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
  }

  function cancelNewTheme() {
    setNewThemeDraft(null)
  }

  function updateNewThemeAccent(accent: string) {
    setNewThemeDraft((current) =>
      current ? syncNewThemeDraftAccent(current, accent) : current,
    )
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
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  /* ── Agent panel helpers ────────────────────────────────── */

  async function loadAgentPanelData() {
    if (!selectedServiceSlug) return
    try {
      const [machines, agents, templates] = await Promise.all([
        fetchMachines(selectedServiceSlug),
        fetchAgents(selectedServiceSlug),
        getAgentTemplates(),
      ])
      setAgentMachines(machines)
      setAgentList(agents)
      setAgentTemplateList(templates)
    } catch {
      // silent — lists will be empty
    }
  }

  function resetAgentCreateForm() {
    setAgentCreateForm({
      name: '',
      displayName: '',
      machineId: '',
      instructions: '',
      model: '',
      maxTurns: 0,
      channelId: '',
      templateId: '',
      sourceAgentId: '',
      envVars: [],
    })
    setAgentCreateMode('scratch')
    setAgentCreateAdvanced(false)
  }

  async function handleAgentCreateFromTemplate(templateId: string) {
    const tpl = agentTemplateList.find((t) => t.id === templateId)
    if (!tpl) return
    setAgentCreateForm((prev) => ({
      ...prev,
      templateId,
      instructions: tpl.config.instructions,
      model: tpl.config.model,
      maxTurns: tpl.config.maxTurns,
      channelId: tpl.config.channelId ?? '',
      envVars: tpl.config.envVars?.map((v) => ({ ...v })) ?? [],
    }))
  }

  async function handleAgentCreateFromAgent(agentId: string) {
    if (!selectedServiceSlug) return
    try {
      const detail = await fetchAgentDetail(selectedServiceSlug, agentId)
      setAgentCreateForm((prev) => ({
        ...prev,
        sourceAgentId: agentId,
        instructions: detail.instructions ?? '',
        model: detail.model ?? '',
        maxTurns: detail.maxTurns ?? 0,
        channelId: detail.channelId ?? '',
        envVars: detail.environmentVariables?.map((v) => ({ ...v })) ?? [],
      }))
    } catch {
      // keep form as-is
    }
  }

  function normalizeEnvVars(vars: { key: string; value: string }[]) {
    return vars
      .map((v) => ({ key: v.key.trim(), value: v.value }))
      .filter((v) => v.key.length > 0)
  }

  async function handleAgentCreate() {
    if (!selectedServiceSlug || !agentCreateForm.name || !agentCreateForm.machineId) return
    setAgentCreateBusy(true)
    try {
      const envNorm = normalizeEnvVars(agentCreateForm.envVars)
      await createAgent(selectedServiceSlug, {
        name: agentCreateForm.name,
        displayName: agentCreateForm.displayName || undefined,
        machineId: agentCreateForm.machineId,
        instructions: agentCreateForm.instructions || undefined,
        model: agentCreateForm.model || undefined,
        maxTurns: agentCreateForm.maxTurns > 0 ? agentCreateForm.maxTurns : undefined,
        channelId: agentCreateForm.channelId || undefined,
        environmentVariables: envNorm.length > 0 ? envNorm : undefined,
      })
      setAgentPanelOpen(false)
      setAgentPanelView('menu')
      resetAgentCreateForm()
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setAgentCreateBusy(false)
    }
  }

  async function handleSaveAsTemplate() {
    const envNorm = normalizeEnvVars(agentCreateForm.envVars)
    const template: AgentTemplate = {
      id: crypto.randomUUID(),
      name: agentCreateForm.name || copy.agentTemplateUntitled,
      source: agentCreateMode === 'agent' ? 'from-agent' : 'custom',
      sourceAgentId: agentCreateMode === 'agent' ? agentCreateForm.sourceAgentId || null : null,
      config: {
        instructions: agentCreateForm.instructions,
        model: agentCreateForm.model,
        maxTurns: agentCreateForm.maxTurns,
        channelId: agentCreateForm.channelId || null,
        envVars: envNorm.length > 0 ? envNorm : null,
      },
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }
    try {
      await saveAgentTemplate(template)
      setAgentTemplateList((prev) => [...prev, template])
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    }
  }

  async function handleDeleteTemplate(id: string) {
    setAgentTemplateBusy(true)
    try {
      await deleteAgentTemplate(id)
      setAgentTemplateList((prev) => prev.filter((t) => t.id !== id))
      if (agentEditTemplate?.id === id) {
        setAgentPanelView('templates')
        setAgentEditTemplate(null)
      }
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setAgentTemplateBusy(false)
    }
  }

  async function handleSaveTemplate() {
    if (!agentEditTemplate) return
    setAgentTemplateBusy(true)
    try {
      const envNorm = normalizeEnvVars(agentEditTemplate.config.envVars ?? [])
      const updated: AgentTemplate = {
        ...agentEditTemplate,
        config: { ...agentEditTemplate.config, envVars: envNorm.length > 0 ? envNorm : null },
        updatedAt: new Date().toISOString(),
      }
      await saveAgentTemplate(updated)
      setAgentTemplateList((prev) => prev.map((t) => (t.id === updated.id ? updated : t)))
      setAgentPanelView('templates')
      setAgentEditTemplate(null)
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setAgentTemplateBusy(false)
    }
  }

  function startNewTemplate() {
    const blank: AgentTemplate = {
      id: crypto.randomUUID(),
      name: '',
      source: 'custom',
      sourceAgentId: null,
      config: { instructions: '', model: '', maxTurns: 0, channelId: null, envVars: null },
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }
    setAgentEditTemplate(blank)
    setAgentPanelView('template-edit')
  }

  async function startTemplateFromAgent() {
    if (!selectedServiceSlug) return
    // Load agents then go to template-edit with source=agent
    try {
      const agents = await fetchAgents(selectedServiceSlug)
      setAgentList(agents)
    } catch {
      // keep existing list
    }
    const blank: AgentTemplate = {
      id: crypto.randomUUID(),
      name: '',
      source: 'from-agent',
      sourceAgentId: null,
      config: { instructions: '', model: '', maxTurns: 0, channelId: null, envVars: null },
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    }
    setAgentEditTemplate(blank)
    setAgentPanelView('template-edit')
  }

  async function handleTemplateImportFromAgent(agentId: string) {
    if (!selectedServiceSlug || !agentEditTemplate) return
    try {
      const detail = await fetchAgentDetail(selectedServiceSlug, agentId)
      setAgentEditTemplate((prev) =>
        prev
          ? {
              ...prev,
              sourceAgentId: agentId,
              name: prev.name || detail.displayName || detail.name,
              config: {
                instructions: detail.instructions ?? '',
                model: detail.model ?? '',
                maxTurns: detail.maxTurns ?? 0,
                channelId: detail.channelId ?? null,
                envVars: detail.environmentVariables?.map((v) => ({ ...v })) ?? null,
              },
            }
          : prev,
      )
    } catch {
      // keep form as-is
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

  function handleMessageReminderDismiss(reminderId: string) {
    const timerId = messageReminderTimersRef.current.get(reminderId)
    if (timerId !== undefined) {
      window.clearTimeout(timerId)
      messageReminderTimersRef.current.delete(reminderId)
    }
    const next = messageRemindersRef.current.filter((r) => r.id !== reminderId)
    messageRemindersRef.current = next
    setMessageReminders(next)
  }

  async function handleMessageReminderOpen(reminder: MessageReminderToast) {
    handleMessageReminderDismiss(reminder.id)
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
      // Optimistic update: immediately reflect the new selection in the UI
      // so the selected state transitions instantly without waiting for IPC.
      setSnapshot((prev) => {
        if (!prev) return prev
        return {
          ...prev,
          service: {
            ...prev.service,
            selectedServerSlug,
            servers: prev.service.servers.map((s) => ({
              ...s,
              selected: s.slug === selectedServerSlug,
            })),
          },
        }
      })
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

    if (!running) {
      // Pre-check: does this server have any machines?
      try {
        setBusyAction('start-service')
        setErrorMessage(null)
        const check = await checkServerMachines(selectedServerSlug)
        if (!check.hasMachines) {
          setBusyAction(null)
          setComputerCreateFlow({
            phase: 'prompt',
            serverSlug: check.serverSlug,
            createUrl: check.createUrl,
            checking: false,
            existingMachineIds: check.machines.map((m) => m.id),
          })
          return
        }
      } catch (error) {
        // If check fails, fall through to normal start (let it handle errors)
        console.warn('Machine check failed, attempting start:', error)
      }
    }

    try {
      if (!busyAction) setBusyAction(running ? 'stop-service' : 'start-service')
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

  async function handleComputerCreateOpen() {
    if (!computerCreateFlow) return
    try {
      await openComputerCreatePage(computerCreateFlow.serverSlug)
      setComputerCreateFlow((prev) =>
        prev ? { ...prev, phase: 'waiting' } : null,
      )
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    }
  }

  async function handleComputerCreateCheck() {
    if (!computerCreateFlow) return
    setComputerCreateFlow((prev) =>
      prev ? { ...prev, checking: true } : null,
    )
    try {
      const check = await checkServerMachines(computerCreateFlow.serverSlug)
      if (check.hasMachines && check.machines.length > 0) {
        // Detect newly created machines by diffing against known IDs
        const existingIds = new Set(computerCreateFlow.existingMachineIds)
        const newMachines = check.machines.filter(
          (m) => !existingIds.has(m.id),
        )
        const targetMachine =
          newMachines.length > 0 ? newMachines[0] : check.machines[0]
        try {
          const info = await prepareDaemonCommand(
            computerCreateFlow.serverSlug,
            targetMachine.id,
          )
          setComputerCreateFlow((prev) =>
            prev
              ? {
                  ...prev,
                  phase: 'command',
                  checking: false,
                  machineId: info.machineId,
                  machineName: info.machineName,
                  daemonCommand: info.command,
                  displayCommand: info.displayCommand,
                }
              : null,
          )
        } catch {
          // Binding/key rotation failed — stay in waiting, don't degrade
          setComputerCreateFlow((prev) =>
            prev ? { ...prev, checking: false } : null,
          )
        }
      } else {
        setComputerCreateFlow((prev) =>
          prev ? { ...prev, checking: false } : null,
        )
      }
    } catch {
      setComputerCreateFlow((prev) =>
        prev ? { ...prev, checking: false } : null,
      )
    }
  }

  async function handleComputerCreateStart() {
    if (!computerCreateFlow) return
    setComputerCreateFlow(null)
    try {
      setBusyAction('start-service')
      setErrorMessage(null)
      await waitForNextPaint()
      const next = await startService(computerCreateFlow.serverSlug)
      startTransition(() => setSnapshot(next))
    } catch (error) {
      setErrorMessage(getErrorMessage(error))
    } finally {
      setBusyAction(null)
    }
  }

  async function handleQuickSend() {
    if (!quickSendTarget || quickSendSending) return
    const hasText = quickSendText.trim().length > 0
    const attachIds = quickSendAttachments.filter((a) => a.id).map((a) => a.id!)
    if (!hasText && attachIds.length === 0) return
    setQuickSendSending(true)
    try {
      await sendMessage(quickSendTarget.serverSlug, quickSendTarget.channelId, quickSendText.trim(), attachIds.length > 0 ? attachIds : undefined)
      setQuickSendText('')
      setQuickSendAttachments([])
    } catch (err) {
      console.error('Failed to send quick message', err)
    } finally {
      setQuickSendSending(false)
    }
  }

  // Reply to message in detail panel (#67)
  async function handleDetailReply() {
    if (!selectedChannel || replySending) return
    const hasText = replyText.trim().length > 0
    const attachIds = replyAttachments.filter((a) => a.id).map((a) => a.id!)
    if (!hasText && attachIds.length === 0) return
    setReplySending(true)
    try {
      const msg = await sendMessage(selectedChannel.serverSlug, selectedChannel.channelId, replyText.trim(), attachIds.length > 0 ? attachIds : undefined)
      setReplyText('')
      setReplyAttachments([])
      setDetailMessages((prev) => [...prev, msg])
      // Auto-scroll to bottom
      requestAnimationFrame(() => {
        if (detailScrollRef.current) {
          detailScrollRef.current.scrollTop = detailScrollRef.current.scrollHeight
        }
      })
    } catch (err) {
      console.error('Failed to send reply', err)
    } finally {
      setReplySending(false)
    }
  }

  // Load older messages in detail panel
  async function handleDetailLoadMore() {
    if (!selectedChannel || detailLoading || !detailHasMore) return
    const firstMsg = detailMessages[0]
    if (!firstMsg) return
    setDetailLoading(true)
    try {
      const fetcher = selectedChannel.itemType === 'thread' ? fetchThreadMessages : fetchChannelMessages
      const resp = await fetcher(selectedChannel.serverSlug, selectedChannel.channelId, { limit: 50, before: firstMsg.id })
      setDetailMessages((prev) => normalizeMessages([...resp.messages, ...prev]))
      setDetailHasMore(resp.hasMore)
    } catch {
      /* ignore */
    } finally {
      setDetailLoading(false)
    }
  }

  // Upload a file attachment for ComposeBox
  async function handleFileUpload(
    file: File,
    serverSlug: string,
    channelId: string | undefined,
    setAttachments: React.Dispatch<React.SetStateAction<{ file: File; uploading: boolean; error?: boolean; id?: string }[]>>,
  ) {
    const entry = { file, uploading: true }
    setAttachments((prev) => [...prev, entry])
    try {
      const buf = await file.arrayBuffer()
      const data = Array.from(new Uint8Array(buf))
      const result = await uploadAttachment(serverSlug, file.name, file.type || 'application/octet-stream', data, channelId)
      setAttachments((prev) =>
        prev.map((a) => (a.file === file ? { ...a, uploading: false, id: result.id } : a)),
      )
    } catch (err) {
      console.error('Upload failed', err)
      // Keep the attachment with error state so the user can see it failed and remove manually
      setAttachments((prev) =>
        prev.map((a) => (a.file === file ? { ...a, uploading: false, error: true } : a)),
      )
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
                    <button
                      key={server.id}
                      className={`service-server-row${selected ? ' selected' : ''}${running ? ' running' : ''}`}
                      type="button"
                      aria-pressed={selected}
                      disabled={serverSelectionDisabled}
                      onClick={() => handleServiceServerSelect(server.slug)}
                      title={server.name}
                    >
                      <span className="service-server-name">{server.name}</span>
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
            </div>
          ) : null}
        </div>

        <button
          type="button"
          className={`titlebar-pill-button titlebar-launch${workspaceLaunching ? ' launching' : ''}`}
          onClick={() => {
            launchButtonAccentRef.current = selectedThemeAccent
            void handleWorkspaceOpen(selectedServiceSlug || undefined)
          }}
          disabled={serviceActionBusy}
          title={stackButtonLabel}
          aria-label={stackButtonLabel}
        >
          {busyAction === 'workspace' ? <SpinnerIcon /> : <EnterIcon />}
          <span>{stackButtonLabel}</span>
        </button>

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

        {selectedServiceRunning ? (
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
        ) : null}

        {/* Agent panel (#83) */}
        <div className="titlebar-agent" ref={agentPanelRef}>
          <button
            type="button"
            className="titlebar-icon-button"
            onClick={() => {
              if (agentPanelOpen) {
                setAgentPanelOpen(false)
                setAgentPanelView('menu')
              } else {
                setAgentPanelOpen(true)
                setAgentPanelView('menu')
                void loadAgentPanelData()
              }
            }}
            aria-expanded={agentPanelOpen}
            title={copy.agentCreate}
            aria-label={copy.agentCreate}
          >
            <BotIcon />
          </button>
          {agentPanelOpen ? (
            <div className="agent-panel" aria-label={copy.agentCreate}>
              {/* Menu view */}
              {agentPanelView === 'menu' ? (
                <div className="agent-menu">
                  <button
                    type="button"
                    className="agent-menu-item"
                    onClick={() => {
                      resetAgentCreateForm()
                      void loadAgentPanelData()
                      setAgentPanelView('create')
                    }}
                  >
                    <span className="agent-menu-icon">+</span>
                    <span>{copy.agentNewAgent}</span>
                  </button>
                  <button
                    type="button"
                    className="agent-menu-item"
                    onClick={() => {
                      void loadAgentPanelData()
                      setAgentPanelView('templates')
                    }}
                  >
                    <span className="agent-menu-icon">⚙</span>
                    <span>{copy.agentTemplates}</span>
                  </button>
                </div>
              ) : null}

              {/* Create view */}
              {agentPanelView === 'create' ? (
                <div className="agent-create">
                  <button
                    type="button"
                    className="agent-back-button"
                    onClick={() => setAgentPanelView('menu')}
                  >
                    {copy.agentBack}
                  </button>
                  <p className="eyebrow">{copy.agentCreateTitle}</p>

                  {/* Mode segmented control */}
                  <div className="agent-field">
                    <label className="agent-label">{copy.agentCreateMode}</label>
                    <div className="agent-segmented" role="radiogroup" aria-label={copy.agentCreateMode}>
                      {(['scratch', 'template', 'agent'] as const).map((mode) => (
                        <button
                          key={mode}
                          type="button"
                          role="radio"
                          aria-checked={agentCreateMode === mode}
                          className={`agent-segment${agentCreateMode === mode ? ' active' : ''}`}
                          onClick={() => {
                            resetAgentCreateForm()
                            setAgentCreateMode(mode)
                          }}
                        >
                          {mode === 'scratch' ? copy.agentModeBlank : mode === 'template' ? copy.agentModeTemplate : copy.agentModeAgent}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Template selector */}
                  {agentCreateMode === 'template' ? (
                    <div className="agent-field">
                      <select
                        className="agent-select"
                        value={agentCreateForm.templateId}
                        onChange={(e) => {
                          const templateId = e.target.value
                          setAgentCreateForm((prev) => ({ ...prev, templateId }))
                          if (templateId) void handleAgentCreateFromTemplate(templateId)
                        }}
                      >
                        <option value="">{copy.agentSelectTemplate}</option>
                        {agentTemplateList.map((t) => (
                          <option key={t.id} value={t.id}>{t.name}</option>
                        ))}
                      </select>
                    </div>
                  ) : null}

                  {/* Agent selector */}
                  {agentCreateMode === 'agent' ? (
                    <div className="agent-field">
                      <select
                        className="agent-select"
                        value={agentCreateForm.sourceAgentId}
                        onChange={(e) => {
                          const agentId = e.target.value
                          setAgentCreateForm((prev) => ({ ...prev, sourceAgentId: agentId }))
                          if (agentId) void handleAgentCreateFromAgent(agentId)
                        }}
                      >
                        <option value="">{copy.agentSelectAgent}</option>
                        {agentList.filter((a) => a.status !== 'deleted').map((a) => (
                          <option key={a.id} value={a.id}>{a.displayName || a.name}</option>
                        ))}
                      </select>
                    </div>
                  ) : null}

                  {/* Name */}
                  <div className="agent-field">
                    <label className="agent-label">{copy.agentName}</label>
                    <input
                      className="agent-input"
                      value={agentCreateForm.name}
                      onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, name: e.target.value }))}
                      placeholder={copy.agentName}
                    />
                  </div>

                  {/* Display name */}
                  <div className="agent-field">
                    <label className="agent-label">{copy.agentDisplayName}</label>
                    <input
                      className="agent-input"
                      value={agentCreateForm.displayName}
                      onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, displayName: e.target.value }))}
                      placeholder={copy.agentDisplayName}
                    />
                  </div>

                  {/* Computer */}
                  <div className="agent-field">
                    <label className="agent-label">{copy.agentComputer}</label>
                    <select
                      className="agent-select"
                      value={agentCreateForm.machineId}
                      onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, machineId: e.target.value }))}
                    >
                      <option value="">{copy.agentSelectComputer}</option>
                      {agentMachines.map((m) => (
                        <option key={m.id} value={m.id}>{m.name} ({m.status})</option>
                      ))}
                    </select>
                  </div>

                  {/* Instructions */}
                  <div className="agent-field">
                    <label className="agent-label">{copy.agentInstructions}</label>
                    <textarea
                      className="agent-textarea"
                      rows={3}
                      value={agentCreateForm.instructions}
                      onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, instructions: e.target.value }))}
                      placeholder={copy.agentInstructions}
                    />
                  </div>

                  {/* Advanced toggle */}
                  <button
                    type="button"
                    className="agent-advanced-toggle"
                    onClick={() => setAgentCreateAdvanced((prev) => !prev)}
                  >
                    {agentCreateAdvanced ? '▾' : '▸'} {copy.agentAdvanced}
                  </button>

                  {agentCreateAdvanced ? (
                    <div className="agent-advanced-fields">
                      <div className="agent-field">
                        <label className="agent-label">{copy.agentModel}</label>
                        <input
                          className="agent-input"
                          value={agentCreateForm.model}
                          onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, model: e.target.value }))}
                          placeholder={copy.agentModel}
                        />
                      </div>
                      <div className="agent-field">
                        <label className="agent-label">{copy.agentMaxTurns}</label>
                        <input
                          className="agent-input"
                          type="number"
                          min={0}
                          value={agentCreateForm.maxTurns || ''}
                          onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, maxTurns: parseInt(e.target.value, 10) || 0 }))}
                          placeholder="0"
                        />
                      </div>
                      <div className="agent-field">
                        <label className="agent-label">{copy.agentChannel}</label>
                        <input
                          className="agent-input"
                          value={agentCreateForm.channelId}
                          onChange={(e) => setAgentCreateForm((prev) => ({ ...prev, channelId: e.target.value }))}
                          placeholder={copy.agentChannel}
                        />
                      </div>

                      {/* Environment Variables */}
                      <div className="agent-field">
                        <label className="agent-label">{copy.agentEnvVars}</label>
                        {agentCreateForm.envVars.map((ev, idx) => (
                          <div key={idx} className="agent-env-row">
                            <input
                              className="agent-input agent-env-key"
                              value={ev.key}
                              onChange={(e) => {
                                const next = [...agentCreateForm.envVars]
                                next[idx] = { ...next[idx], key: e.target.value }
                                setAgentCreateForm((prev) => ({ ...prev, envVars: next }))
                              }}
                              placeholder={copy.agentEnvVarsKey}
                            />
                            <input
                              className="agent-input agent-env-value"
                              value={ev.value}
                              onChange={(e) => {
                                const next = [...agentCreateForm.envVars]
                                next[idx] = { ...next[idx], value: e.target.value }
                                setAgentCreateForm((prev) => ({ ...prev, envVars: next }))
                              }}
                              placeholder={copy.agentEnvVarsValue}
                            />
                            <button
                              type="button"
                              className="agent-env-delete"
                              onClick={() => {
                                const next = agentCreateForm.envVars.filter((_, i) => i !== idx)
                                setAgentCreateForm((prev) => ({ ...prev, envVars: next }))
                              }}
                            >
                              ✕
                            </button>
                          </div>
                        ))}
                        <button
                          type="button"
                          className="agent-env-add"
                          onClick={() =>
                            setAgentCreateForm((prev) => ({
                              ...prev,
                              envVars: [...prev.envVars, { key: '', value: '' }],
                            }))
                          }
                        >
                          {copy.agentEnvVarsAdd}
                        </button>
                      </div>
                    </div>
                  ) : null}

                  {/* Actions */}
                  <div className="agent-actions">
                    <button
                      type="button"
                      className="tiny-button muted"
                      onClick={handleSaveAsTemplate}
                    >
                      {copy.agentSaveTemplate}
                    </button>
                    <div className="agent-actions-right">
                      <button
                        type="button"
                        className="tiny-button muted"
                        onClick={() => {
                          setAgentPanelView('menu')
                          resetAgentCreateForm()
                        }}
                      >
                        {copy.agentCancelBtn}
                      </button>
                      <button
                        type="button"
                        className="tiny-button accent"
                        onClick={handleAgentCreate}
                        disabled={agentCreateBusy || !agentCreateForm.name || !agentCreateForm.machineId}
                      >
                        {agentCreateBusy ? copy.agentCreating : copy.agentCreateBtn}
                      </button>
                    </div>
                  </div>
                </div>
              ) : null}

              {/* Templates view */}
              {agentPanelView === 'templates' ? (
                <div className="agent-templates">
                  <button
                    type="button"
                    className="agent-back-button"
                    onClick={() => setAgentPanelView('menu')}
                  >
                    {copy.agentBack}
                  </button>
                  <p className="eyebrow">{copy.agentTemplateTitle}</p>

                  {agentTemplateList.length === 0 ? (
                    <p className="agent-empty">{copy.agentNoTemplates}</p>
                  ) : (
                    <div className="agent-template-list">
                      {agentTemplateList.map((tpl) => (
                        <div key={tpl.id} className="agent-template-card">
                          <div className="agent-template-info">
                            <span className="agent-template-name">{tpl.name || copy.agentTemplateUntitled}</span>
                            <span className="agent-template-meta">
                              {tpl.config.model || '—'} · {tpl.config.maxTurns || '∞'} {copy.agentTemplateTurns}
                            </span>
                          </div>
                          <div className="agent-template-card-actions">
                            <button
                              type="button"
                              className="tiny-button muted"
                              onClick={() => {
                                setAgentEditTemplate({ ...tpl })
                                setAgentPanelView('template-edit')
                              }}
                            >
                              ✏️
                            </button>
                            <button
                              type="button"
                              className="tiny-button muted"
                              onClick={() => void handleDeleteTemplate(tpl.id)}
                              disabled={agentTemplateBusy}
                            >
                              🗑
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}

                  <div className="agent-template-add-actions">
                    <button
                      type="button"
                      className="agent-menu-item compact"
                      onClick={startNewTemplate}
                    >
                      {copy.agentNewTemplate}
                    </button>
                    <button
                      type="button"
                      className="agent-menu-item compact"
                      onClick={() => void startTemplateFromAgent()}
                    >
                      {copy.agentFromAgent}
                    </button>
                  </div>
                </div>
              ) : null}

              {/* Template edit view */}
              {agentPanelView === 'template-edit' && agentEditTemplate ? (
                <div className="agent-template-edit">
                  <button
                    type="button"
                    className="agent-back-button"
                    onClick={() => {
                      setAgentPanelView('templates')
                      setAgentEditTemplate(null)
                    }}
                  >
                    {copy.agentBack}
                  </button>
                  <p className="eyebrow">{copy.agentEditTemplate}</p>

                  {/* Import from agent (for source=agent templates) */}
                  {agentEditTemplate.source === 'from-agent' ? (
                    <div className="agent-field">
                      <label className="agent-label">{copy.agentSelectAgent}</label>
                      <select
                        className="agent-select"
                        value={agentEditTemplate.sourceAgentId ?? ''}
                        onChange={(e) => {
                          const agentId = e.target.value
                          if (agentId) void handleTemplateImportFromAgent(agentId)
                        }}
                      >
                        <option value="">{copy.agentSelectAgent}</option>
                        {agentList.filter((a) => a.status !== 'deleted').map((a) => (
                          <option key={a.id} value={a.id}>{a.displayName || a.name}</option>
                        ))}
                      </select>
                    </div>
                  ) : null}

                  <div className="agent-field">
                    <label className="agent-label">{copy.agentTemplateName}</label>
                    <input
                      className="agent-input"
                      value={agentEditTemplate.name}
                      onChange={(e) =>
                        setAgentEditTemplate((prev) =>
                          prev ? { ...prev, name: e.target.value } : prev,
                        )
                      }
                      placeholder={copy.agentTemplateName}
                    />
                  </div>

                  <div className="agent-field">
                    <label className="agent-label">{copy.agentInstructions}</label>
                    <textarea
                      className="agent-textarea"
                      rows={3}
                      value={agentEditTemplate.config.instructions}
                      onChange={(e) =>
                        setAgentEditTemplate((prev) =>
                          prev
                            ? { ...prev, config: { ...prev.config, instructions: e.target.value } }
                            : prev,
                        )
                      }
                      placeholder={copy.agentInstructions}
                    />
                  </div>

                  <div className="agent-field">
                    <label className="agent-label">{copy.agentModel}</label>
                    <input
                      className="agent-input"
                      value={agentEditTemplate.config.model}
                      onChange={(e) =>
                        setAgentEditTemplate((prev) =>
                          prev
                            ? { ...prev, config: { ...prev.config, model: e.target.value } }
                            : prev,
                        )
                      }
                      placeholder={copy.agentModel}
                    />
                  </div>

                  <div className="agent-field">
                    <label className="agent-label">{copy.agentMaxTurns}</label>
                    <input
                      className="agent-input"
                      type="number"
                      min={0}
                      value={agentEditTemplate.config.maxTurns || ''}
                      onChange={(e) =>
                        setAgentEditTemplate((prev) =>
                          prev
                            ? {
                                ...prev,
                                config: {
                                  ...prev.config,
                                  maxTurns: parseInt(e.target.value, 10) || 0,
                                },
                              }
                            : prev,
                        )
                      }
                      placeholder="0"
                    />
                  </div>

                  <div className="agent-field">
                    <label className="agent-label">{copy.agentChannel}</label>
                    <input
                      className="agent-input"
                      value={agentEditTemplate.config.channelId ?? ''}
                      onChange={(e) =>
                        setAgentEditTemplate((prev) =>
                          prev
                            ? {
                                ...prev,
                                config: {
                                  ...prev.config,
                                  channelId: e.target.value || null,
                                },
                              }
                            : prev,
                        )
                      }
                      placeholder={copy.agentChannel}
                    />
                  </div>

                  {/* Environment Variables */}
                  <div className="agent-field">
                    <label className="agent-label">{copy.agentEnvVars}</label>
                    {(agentEditTemplate.config.envVars ?? []).map((ev, idx) => (
                      <div key={idx} className="agent-env-row">
                        <input
                          className="agent-input agent-env-key"
                          value={ev.key}
                          onChange={(e) =>
                            setAgentEditTemplate((prev) => {
                              if (!prev) return prev
                              const next = [...(prev.config.envVars ?? [])]
                              next[idx] = { ...next[idx], key: e.target.value }
                              return { ...prev, config: { ...prev.config, envVars: next } }
                            })
                          }
                          placeholder={copy.agentEnvVarsKey}
                        />
                        <input
                          className="agent-input agent-env-value"
                          value={ev.value}
                          onChange={(e) =>
                            setAgentEditTemplate((prev) => {
                              if (!prev) return prev
                              const next = [...(prev.config.envVars ?? [])]
                              next[idx] = { ...next[idx], value: e.target.value }
                              return { ...prev, config: { ...prev.config, envVars: next } }
                            })
                          }
                          placeholder={copy.agentEnvVarsValue}
                        />
                        <button
                          type="button"
                          className="agent-env-delete"
                          onClick={() =>
                            setAgentEditTemplate((prev) => {
                              if (!prev) return prev
                              const next = (prev.config.envVars ?? []).filter((_, i) => i !== idx)
                              return { ...prev, config: { ...prev.config, envVars: next.length > 0 ? next : null } }
                            })
                          }
                        >
                          ✕
                        </button>
                      </div>
                    ))}
                    <button
                      type="button"
                      className="agent-env-add"
                      onClick={() =>
                        setAgentEditTemplate((prev) => {
                          if (!prev) return prev
                          return {
                            ...prev,
                            config: {
                              ...prev.config,
                              envVars: [...(prev.config.envVars ?? []), { key: '', value: '' }],
                            },
                          }
                        })
                      }
                    >
                      {copy.agentEnvVarsAdd}
                    </button>
                  </div>

                  <div className="agent-actions">
                    <button
                      type="button"
                      className="tiny-button muted"
                      onClick={() => {
                        setAgentPanelView('templates')
                        setAgentEditTemplate(null)
                      }}
                    >
                      {copy.agentCancelBtn}
                    </button>
                    <button
                      type="button"
                      className="tiny-button accent"
                      onClick={handleSaveTemplate}
                      disabled={agentTemplateBusy || !agentEditTemplate.name}
                    >
                      {copy.agentTemplateSave}
                    </button>
                  </div>
                </div>
              ) : null}
            </div>
          ) : null}
        </div>

        <div className="tauri-titlebar-drag" data-tauri-drag-region />

        {/* Unified Appearance panel (#75/#76 Phase 2) */}
        <div className="titlebar-appearance" ref={stylePanelRef}>
          <button
            type="button"
            className="titlebar-theme-button"
            onClick={() => setStylePanelOpen((open) => !open)}
            aria-expanded={stylePanelOpen}
            title={copy.themeStyle}
            aria-label={copy.themeStyle}
            style={{ '--current-accent': selectedThemeAccent } as CSSProperties}
          >
            <span className="titlebar-theme-swatch" aria-hidden="true" />
          </button>
          {stylePanelOpen ? (
            <div className="appearance-panel" aria-label={copy.themeStyle}>
              {/* Style presets — compact color blocks */}
              <div className="appearance-section">
                <p className="eyebrow">{copy.themeStyle}</p>
                <div className="appearance-style-row" role="radiogroup" aria-label={copy.themeStyle}>
                  {snapshot.themeStyles.map((style) => {
                    const selected = style.id === snapshot.styleScheme || (style.id === 'original' && activeIsOriginal)
                    return (
                      <button
                        key={style.id}
                        type="button"
                        className={`appearance-style-block${selected ? ' selected' : ''}${busyAction === `style:${style.id}` ? ' busy' : ''}`}
                        role="radio"
                        aria-checked={selected}
                        aria-label={getThemeStyleName(style, copy)}
                        title={getThemeStyleName(style, copy)}
                        style={{
                          '--block-a': style.preview[0],
                          '--block-b': style.preview[1],
                          '--block-c': style.preview[2],
                        } as CSSProperties}
                        onClick={() => handleThemeStyleChange(style.id)}
                        disabled={busyAction?.startsWith('style:')}
                      >
                        <span className="appearance-style-preview" aria-hidden="true" />
                        {selected && <span className="appearance-style-check" aria-hidden="true" />}
                      </button>
                    )
                  })}
                </div>
                <span className="appearance-style-name">{getThemeStyleName(
                  snapshot.themeStyles.find((s) => s.id === snapshot.styleScheme) ??
                  snapshot.themeStyles.find((s) => s.id === 'original') ??
                  snapshot.themeStyles[0],
                  copy,
                )}</span>
              </div>

              <hr className="appearance-divider" />

              {/* Accent color dots */}
              <div className="appearance-section">
                <p className="eyebrow">{copy.themeColor}</p>
                <div className="appearance-accent-row" role="radiogroup" aria-label={copy.themeColor}>
                  {snapshot.themes.map((theme) => {
                    const customTheme = snapshot.customThemes.find((item) => item.id === theme.id)
                    const selected = theme.id === snapshot.colorScheme
                    const swatch = customTheme?.accent ?? DEFAULT_NEW_THEME_ACCENT
                    const builtIn = !customTheme
                    return (
                      <button
                        key={theme.id}
                        type="button"
                        className={`appearance-accent-dot${selected ? ' selected' : ''}`}
                        role="radio"
                        aria-checked={selected}
                        aria-label={builtIn ? copy.themeDefaultColorName : (customTheme?.name ?? theme.name)}
                        title={builtIn ? copy.themeDefaultColorName : (customTheme?.name ?? theme.name)}
                        style={{ '--dot-color': swatch } as CSSProperties}
                        onClick={() => handleThemeChange(theme.id)}
                        disabled={busyAction === `theme:${theme.id}`}
                      />
                    )
                  })}
                  <button
                    type="button"
                    className="appearance-accent-add"
                    onClick={startNewTheme}
                    disabled={Boolean(newThemeDraft) || busyAction === 'create-theme'}
                    aria-label={copy.themeNewLabel}
                    title={copy.themeNewLabel}
                  >
                    +
                  </button>
                </div>
                {/* Inline edit/delete for selected custom theme */}
                {(() => {
                  const selectedTheme = snapshot.themes.find((t) => t.id === snapshot.colorScheme)
                  const customTheme = selectedTheme ? snapshot.customThemes.find((c) => c.id === selectedTheme.id) : null
                  if (!customTheme) return null
                  return (
                    <div className="appearance-custom-actions">
                      <input
                        type="color"
                        className="appearance-accent-picker"
                        value={customTheme.accent}
                        onChange={(e) => void handleAccentChange(customTheme.id, e.target.value)}
                        aria-label={copy.customThemeAccentAria}
                        title={copy.customThemeAccentAria}
                      />
                      <button
                        type="button"
                        className="tiny-button muted"
                        onClick={() => void handleDeleteTheme(customTheme.id)}
                        disabled={busyAction === `delete:${customTheme.id}`}
                      >
                        {copy.themeDelete}
                      </button>
                    </div>
                  )
                })()}
              </div>

              {/* New theme draft — compact */}
              {newThemeDraft ? (
                <div className="appearance-new-theme">
                  <hr className="appearance-divider" />
                  <div className="appearance-section">
                    <p className="eyebrow">{copy.themeNewTitle}</p>
                    <div className="appearance-new-theme-row">
                      <input
                        type="color"
                        className="appearance-accent-picker"
                        value={newThemeDraft.accent}
                        onChange={(e) => updateNewThemeAccent(e.target.value)}
                        aria-label={copy.customThemeAccentAria}
                      />
                      <input
                        ref={newNameInputRef}
                        className="appearance-name-input"
                        value={newThemeDraft.name}
                        onChange={(event) =>
                          setNewThemeDraft((current) =>
                            current ? { ...current, name: event.target.value } : current,
                          )
                        }
                        placeholder={copy.customThemeNamePlaceholder}
                        aria-label={copy.themeNewTitle}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter' && !event.nativeEvent.isComposing && event.keyCode !== 229) {
                            event.preventDefault()
                            void handleCreateTheme()
                          } else if (event.key === 'Escape') {
                            event.preventDefault()
                            cancelNewTheme()
                          }
                        }}
                      />
                    </div>
                    <div className="appearance-new-theme-actions">
                      <button
                        className="tiny-button accent"
                        type="button"
                        onClick={handleCreateTheme}
                        disabled={busyAction === 'create-theme'}
                      >
                        {busyAction === 'create-theme' ? copy.creatingTheme : copy.themeCreate}
                      </button>
                      <button
                        className="tiny-button muted"
                        type="button"
                        onClick={cancelNewTheme}
                      >
                        {copy.themeCancel}
                      </button>
                    </div>
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
        <div className="titlebar-release-wrap" ref={releaseNotesRef}>
          <button
            type="button"
            className={`status-chip titlebar-version${releaseUpdateAvailable ? ' warm' : ''}${releaseState.error ? ' error' : ''}`}
            title={releaseStatusTitle}
            aria-expanded={releaseNotesOpen}
            onClick={() => setReleaseNotesOpen((open) => !open)}
          >
            v{snapshot.updates.currentVersion}
            {releaseStatusLabel ? ` · ${releaseStatusLabel}` : ''}
          </button>
          {releaseNotesOpen ? (
            <div className="release-notes-popover" role="dialog" aria-label={copy.releaseCheck}>
              <header className="release-notes-head">
                <div className="release-notes-title">
                  <span className="eyebrow">{copy.releaseCheck}</span>
                  <strong>{releaseState.latest?.version ? `v${releaseState.latest.version}` : `v${snapshot.updates.currentVersion}`}</strong>
                </div>
                <button
                  type="button"
                  className="titlebar-icon-button"
                  onClick={() => setReleaseNotesOpen(false)}
                  aria-label={copy.close}
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                </button>
              </header>
              {releaseState.latest?.date ? (
                <p className="release-notes-date">{copy.published}: {new Date(releaseState.latest.date).toLocaleDateString()}</p>
              ) : null}
              <div className="release-notes-body">
                {releaseState.latest?.body
                  ? releaseState.latest.body
                  : copy.noReleaseNotes}
              </div>
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
              ) : (
                <span className="release-notes-status">{copy.current}</span>
              )}
            </div>
          ) : null}
        </div>
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

        <div className="inbox-layout">
          {/* Left — Server-Grouped Feed */}
          <aside className="inbox-sidebar" aria-label={copy.inbox}>
            <div className="inbox-sidebar-header">
              <input
                type="search"
                className="inbox-search"
                placeholder={copy.inboxSearch}
                value={inboxSearch}
                onChange={(e) => setInboxSearch(e.target.value)}
              />
              <div className="inbox-filter-bar" role="tablist">
                {(['all', 'unread'] as const).map((filter) => {
                  const label = filter === 'all' ? copy.inboxAll : copy.inboxUnreadLabel
                  // Only show badge for the active filter (API handles filtering)
                  const count = filter === inboxFilter
                    ? unifiedItems.filter((i) => i.unreadCount > 0).length
                    : 0
                  return (
                    <button
                      key={filter}
                      type="button"
                      role="tab"
                      className={`inbox-filter-chip${inboxFilter === filter ? ' active' : ''}`}
                      aria-selected={inboxFilter === filter}
                      onClick={() => setInboxFilter(filter)}
                    >
                      {label}
                      {count > 0 ? <span className="inbox-filter-badge">{count}</span> : null}
                    </button>
                  )
                })}
              </div>
            </div>
            <div className="inbox-list" role="listbox">
              {(() => {
                if (inboxLoading) {
                  return <InboxSkeleton />
                }

                const normalizedSearch = inboxSearch.trim().toLowerCase()

                // 1. Search filter (API handles filter tab via server-side filtering)
                let items = unifiedItems
                if (normalizedSearch) {
                  items = items.filter((i) => {
                    const name = i.channelName.toLowerCase()
                    const sender = (i.lastSenderName ?? '').toLowerCase()
                    const preview = (i.lastPreview ?? '').toLowerCase()
                    return name.includes(normalizedSearch) || sender.includes(normalizedSearch) || preview.includes(normalizedSearch)
                  })
                }

                // 3. Group by server, sort each group by lastMessageAt desc
                const serverMap = new Map<string, { serverName: string; items: typeof items }>()
                for (const item of items) {
                  const group = serverMap.get(item.serverSlug)
                  if (group) {
                    group.items.push(item)
                  } else {
                    serverMap.set(item.serverSlug, { serverName: item.serverName, items: [item] })
                  }
                }
                // Sort items within each server by lastMessageAt desc
                for (const group of serverMap.values()) {
                  group.items.sort((a, b) => (b.lastMessageAt ?? '').localeCompare(a.lastMessageAt ?? ''))
                }

                const serverGroups = Array.from(serverMap.entries())
                if (serverGroups.length === 0) {
                  return (
                    <div className="inbox-list-empty">
                      <p className="inline-note">{copy.inboxNoUnread}</p>
                    </div>
                  )
                }

                return serverGroups.map(([slug, group]) => {
                  const isExpanded = expandedServers.has(slug)
                  const displayLimit = isExpanded ? 10 : 5
                  const visibleItems = group.items.slice(0, displayLimit)
                  const hasMore = group.items.length > displayLimit
                  const serverUnread = group.items.reduce((sum, i) => sum + i.unreadCount, 0)

                  return (
                    <div key={slug} className="inbox-server-group">
                      <div className="inbox-feed-server-header">
                        <span className="inbox-feed-server-name">{group.serverName}</span>
                        {serverUnread > 0 ? (
                          <span className="inbox-feed-server-badge">{serverUnread}</span>
                        ) : null}
                      </div>
                      {visibleItems.map((item) => {
                        const isSelected = selectedChannel?.serverSlug === item.serverSlug && selectedChannel?.channelId === item.channelId
                        const sourceLabel = item.type === 'dm'
                          ? `@${item.displayName ?? item.channelName}`
                          : item.channelName
                        return (
                          <button
                            key={item.id}
                            type="button"
                            className={`inbox-feed-item-v2${isSelected ? ' selected' : ''}${item.unreadCount > 0 ? ' unread' : ''}`}
                            onClick={() => setSelectedChannel({ serverSlug: item.serverSlug, channelId: item.channelId, itemType: item.type })}
                          >
                            <div className="inbox-feed-item-header">
                              <span className="inbox-feed-item-source">{sourceLabel}</span>
                              {item.lastMessageAt ? (
                                <span className="inbox-feed-item-time">{formatRelativeTime(item.lastMessageAt)}</span>
                              ) : null}
                            </div>
                            {item.lastPreview ? (
                              <p className="inbox-feed-item-preview">
                                {item.lastSenderName ? (
                                  <><span className="inbox-feed-item-sender">{item.lastSenderName}:</span> {item.lastPreview}</>
                                ) : item.lastPreview}
                              </p>
                            ) : null}
                            {item.replyCount != null && item.replyCount > 0 ? (
                              <span className="inbox-feed-item-replies">{item.replyCount} {copy.inboxReplies}</span>
                            ) : null}
                            {item.unreadCount > 0 ? (
                              <span className="inbox-feed-item-unread">{item.unreadCount} {copy.inboxUnreadLabel}</span>
                            ) : null}
                          </button>
                        )
                      })}
                      {hasMore ? (
                        <button
                          type="button"
                          className="inbox-feed-expand"
                          onClick={() => {
                            setExpandedServers((prev) => {
                              const next = new Set(prev)
                              if (isExpanded) next.delete(slug)
                              else next.add(slug)
                              return next
                            })
                          }}
                        >
                          {isExpanded ? copy.inboxCollapse : copy.inboxExpandMore}
                          <ChevronIcon direction={isExpanded ? 'up' : 'down'} />
                        </button>
                      ) : null}
                    </div>
                  )
                })
              })()}
              {inboxHasMore && !inboxLoading ? (
                <button
                  type="button"
                  className="inbox-feed-expand"
                  onClick={() => void handleLoadMore()}
                  disabled={inboxLoadingMore}
                >
                  {inboxLoadingMore ? <SpinnerIcon /> : copy.inboxExpandMore}
                </button>
              ) : null}
            </div>
          </aside>

          {/* Right — Context Area */}
          <div className="inbox-content">
            {selectedChannel ? (
              <div className="inbox-message-view">
                {/* Header */}
                <div className="inbox-message-header">
                  <button
                    type="button"
                    className="inbox-back-button"
                    onClick={() => setSelectedChannel(null)}
                    aria-label={copy.inboxBack}
                    title={copy.inboxBack}
                  >
                    <ChevronIcon direction="left" />
                  </button>
                  <span className="inbox-message-title">
                    {(() => {
                      const item = unifiedItems.find(
                        (i) => i.serverSlug === selectedChannel.serverSlug && i.channelId === selectedChannel.channelId,
                      )
                      if (!item) return copy.inboxConversation
                      const serverLabel = snapshot?.service.servers.find((s) => s.slug === selectedChannel.serverSlug)?.name ?? ''
                      if (item.type === 'thread') return `Thread in ${item.channelName} (${serverLabel})`
                      if (item.type === 'dm') return `@ ${item.displayName ?? item.channelName} (${serverLabel})`
                      return `${item.channelName} (${serverLabel})`
                    })()}
                  </span>
                  <button
                    type="button"
                    className="inbox-message-close"
                    onClick={() => setSelectedChannel(null)}
                    aria-label={copy.close}
                    title={copy.close}
                  >
                    <XIcon />
                  </button>
                </div>
                {/* Message list */}
                <div
                  className="inbox-detail-messages"
                  ref={detailScrollRef}
                  onScroll={(e) => {
                    const el = e.currentTarget
                    detailAutoScrollRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40
                  }}
                >
                  {detailHasMore && (
                    <button
                      type="button"
                      className="inbox-detail-load-more"
                      onClick={() => void handleDetailLoadMore()}
                      disabled={detailLoading}
                    >
                      {detailLoading ? <SpinnerIcon /> : copy.inboxExpandMore}
                    </button>
                  )}
                  {detailLoading && detailMessages.length === 0 ? (
                    <div className="inbox-detail-loading"><SpinnerIcon /></div>
                  ) : detailMessages.length === 0 ? (
                    <div className="inbox-detail-empty">No messages yet</div>
                  ) : (
                    detailMessages.map((msg, idx) => {
                      const prev = idx > 0 ? detailMessages[idx - 1] : null
                      const sameAuthor = prev?.senderId === msg.senderId && prev?.senderId != null
                      const withinWindow = prev
                        ? Math.abs(new Date(msg.createdAt).getTime() - new Date(prev.createdAt).getTime()) < 300_000
                        : false
                      const compact = sameAuthor && withinWindow
                      const serverUrl = snapshot?.service.serverUrl
                        ? snapshot.service.serverUrl.replace(/\/+$/, '')
                        : 'https://api.slock.ai'

                      return (
                        <div
                          key={msg.id}
                          className={`inbox-detail-msg${compact ? ' compact' : ''}`}
                        >
                          {!compact && (
                            <div className="inbox-detail-msg-header">
                              <span className="inbox-detail-msg-sender">
                                @{msg.senderDisplayName ?? msg.senderName ?? 'Unknown'}
                              </span>
                              <span className="inbox-detail-msg-time">
                                {relativeTime(msg.createdAt)}
                              </span>
                            </div>
                          )}
                          <div
                            className="inbox-detail-msg-content"
                            dangerouslySetInnerHTML={{ __html: renderMarkdown(msg.content) }}
                          />
                          {msg.attachments.length > 0 && (
                            <div className="inbox-detail-msg-attachments">
                              {msg.attachments.map((att) => (
                                <MessageAttachmentView key={att.id} att={att} serverUrl={serverUrl} />
                              ))}
                            </div>
                          )}
                        </div>
                      )
                    })
                  )}
                </div>
                {/* Reply compose box */}
                <div className="inbox-detail-reply">
                  <ComposeBox
                    text={replyText}
                    setText={setReplyText}
                    placeholder={`Reply to ${unifiedItems.find(
                      (i) => i.serverSlug === selectedChannel.serverSlug && i.channelId === selectedChannel.channelId,
                    )?.channelName ?? 'conversation'}…`}
                    sending={replySending}
                    onSend={() => void handleDetailReply()}
                    attachments={replyAttachments}
                    onFileSelect={(files) => {
                      for (const file of Array.from(files)) {
                        void handleFileUpload(file, selectedChannel.serverSlug, selectedChannel.channelId, setReplyAttachments)
                      }
                    }}
                    onRemoveAttachment={(file) => setReplyAttachments((prev) => prev.filter((a) => a.file !== file))}
                  />
                </div>
              </div>
            ) : (
              /* Quick Send — redesigned (#68) */
              <div className="inbox-quick-send">
                <div className="inbox-quick-send-inner">
                  <p className="eyebrow">{copy.inboxQuickSend}</p>
                  <ComposeBox
                    text={quickSendText}
                    setText={setQuickSendText}
                    placeholder={copy.inboxComposePlaceholder}
                    sending={quickSendSending}
                    onSend={() => void handleQuickSend()}
                    attachments={quickSendAttachments}
                    onFileSelect={(files) => {
                      if (!quickSendTarget) return
                      for (const file of Array.from(files)) {
                        void handleFileUpload(file, quickSendTarget.serverSlug, quickSendTarget.channelId, setQuickSendAttachments)
                      }
                    }}
                    onRemoveAttachment={(file) => setQuickSendAttachments((prev) => prev.filter((a) => a.file !== file))}
                    disabled={!quickSendTarget}
                  />
                  {/* Server + Channel/DM selectors on same line */}
                  <div className="inbox-quick-send-selectors">
                    {/* Server selector — hidden when only one server */}
                    {(() => {
                      const allSlugs = new Set([
                        ...serverChannelGroups.map((g) => g.serverSlug),
                        ...serverDmGroups.map((g) => g.serverSlug),
                      ])
                      if (allSlugs.size <= 1) return null
                      const selectedServerName = [...serverChannelGroups, ...serverDmGroups].find(
                        (g) => g.serverSlug === selectedQuickServer,
                      )?.serverName
                      return (
                        <div className="inbox-target-half">
                          <div className="inbox-target-selector">
                            <button
                              type="button"
                              className="inbox-target-button"
                              onClick={() => {
                                setQuickSendServerOpen((o) => !o)
                                setQuickSendTargetOpen(false)
                              }}
                            >
                              {selectedServerName ?? 'Server'}
                              <ChevronIcon direction={quickSendServerOpen ? 'up' : 'down'} />
                            </button>
                            {quickSendServerOpen && (
                              <div className="inbox-target-dropdown">
                                {[...allSlugs].map((slug) => {
                                  const name = [...serverChannelGroups, ...serverDmGroups].find(
                                    (g) => g.serverSlug === slug,
                                  )?.serverName ?? slug
                                  return (
                                    <button
                                      key={slug}
                                      type="button"
                                      className={`inbox-target-option${selectedQuickServer === slug ? ' selected' : ''}`}
                                      onClick={() => {
                                        setSelectedQuickServer(slug)
                                        // Reset target when server changes
                                        setQuickSendTarget(null)
                                        setQuickSendText('')
                                        setQuickSendAttachments([])
                                        setQuickSendServerOpen(false)
                                      }}
                                    >
                                      {name}
                                    </button>
                                  )
                                })}
                              </div>
                            )}
                          </div>
                        </div>
                      )
                    })()}
                    {/* Channel / DM selector — filtered by selected server */}
                    <div className="inbox-target-half">
                      <div className="inbox-target-selector">
                        <button
                          type="button"
                          className="inbox-target-button"
                          onClick={() => {
                            setQuickSendTargetOpen((o) => !o)
                            setQuickSendServerOpen(false)
                          }}
                          disabled={!selectedQuickServer}
                        >
                          {quickSendTarget ? quickSendTarget.label : copy.inboxSelectTarget}
                          <ChevronIcon direction={quickSendTargetOpen ? 'up' : 'down'} />
                        </button>
                        {quickSendTargetOpen && selectedQuickServer ? (
                          <div className="inbox-target-dropdown">
                            {serverChannelGroups
                              .filter((g) => g.serverSlug === selectedQuickServer)
                              .map((group) => (
                                <div key={group.serverSlug} className="inbox-target-group">
                                  <p className="inbox-target-group-name">Channels</p>
                                  {group.channels.map((ch) => (
                                    <button
                                      key={ch.id}
                                      type="button"
                                      className="inbox-target-option"
                                      onClick={() => {
                                        setQuickSendTarget({ serverSlug: group.serverSlug, channelId: ch.id, label: `#${ch.name}` })
                                        setQuickSendTargetOpen(false)
                                      }}
                                    >
                                      #{ch.name}
                                    </button>
                                  ))}
                                </div>
                              ))}
                            {serverDmGroups
                              .filter((g) => g.serverSlug === selectedQuickServer)
                              .map((group) =>
                                group.dms.length > 0 ? (
                                  <div key={`dm-${group.serverSlug}`} className="inbox-target-group">
                                    <p className="inbox-target-group-name">Direct Messages</p>
                                    {group.dms.map((dm) => (
                                      <button
                                        key={dm.id}
                                        type="button"
                                        className="inbox-target-option"
                                        onClick={() => {
                                          setQuickSendTarget({ serverSlug: group.serverSlug, channelId: dm.id, label: `@${dm.displayName ?? dm.name}` })
                                          setQuickSendTargetOpen(false)
                                        }}
                                      >
                                        @{dm.displayName ?? dm.name}
                                      </button>
                                    ))}
                                  </div>
                                ) : null,
                              )}
                          </div>
                        ) : null}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </section>

      {messageReminders.length > 0 ? (
        <div className="message-reminder-stack">
          {messageReminders.map((reminder) => (
            <section
              key={reminder.id}
              className="message-reminder-toast"
              role="status"
              aria-live="polite"
              aria-label={copy.messageReminderTitle}
            >
              <button
                type="button"
                className="message-reminder-main"
                onClick={() => void handleMessageReminderOpen(reminder)}
                title={`${copy.messageReminderOpen} ${reminder.serverName}`}
              >
                <span className="message-reminder-kicker">{copy.messageReminderTitle}</span>
                <span className="message-reminder-title">
                  {reminder.senderName} · {reminder.serverName}
                </span>
                <span className="message-reminder-body">{reminder.contentPreview}</span>
              </button>
              <button
                type="button"
                className="message-reminder-close"
                onClick={() => handleMessageReminderDismiss(reminder.id)}
                aria-label={copy.messageReminderDismiss}
                title={copy.messageReminderDismiss}
              >
                ×
              </button>
            </section>
          ))}
        </div>
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
                      if (event.key === 'Enter' && !event.nativeEvent.isComposing && event.keyCode !== 229) {
                        event.preventDefault()
                        handleServiceLogMatchStep(event.shiftKey ? -1 : 1)
                      }
                    }}
                    placeholder={copy.serverLogSearch}
                    aria-label={copy.serverLogSearch}
                    disabled={!serviceLogViewer.snapshot || serviceLogViewer.loading}
                  />
                </label>
                <span className="service-chip service-log-count">
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

      {computerCreateFlow ? (
        <section
          className="computer-create-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              setComputerCreateFlow(null)
            }
          }}
        >
          <section
            className="computer-create-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="computer-create-title"
          >
            <header className="computer-create-head">
              <h2 id="computer-create-title">{copy.noComputerTitle}</h2>
              <button
                className="icon-action-button compact"
                type="button"
                onClick={() => setComputerCreateFlow(null)}
                aria-label={copy.close}
                title={copy.close}
              >
                <XIcon />
              </button>
            </header>

            <div className="computer-create-body">
              {computerCreateFlow.phase === 'prompt' ? (
                <>
                  <p className="computer-create-message">{copy.noComputerMessage}</p>
                  <button
                    type="button"
                    className="computer-create-action"
                    onClick={handleComputerCreateOpen}
                  >
                    {copy.noComputerCreate}
                  </button>
                </>
              ) : computerCreateFlow.phase === 'waiting' ? (
                <>
                  <p className="computer-create-message">{copy.noComputerWaiting}</p>
                  <button
                    type="button"
                    className="computer-create-action"
                    onClick={handleComputerCreateCheck}
                    disabled={computerCreateFlow.checking}
                  >
                    {computerCreateFlow.checking
                      ? copy.noComputerChecking
                      : copy.noComputerRefresh}
                  </button>
                </>
              ) : computerCreateFlow.phase === 'ready' ? (
                <>
                  <p className="computer-create-message computer-create-ready">
                    {copy.noComputerReady}
                  </p>
                  <button
                    type="button"
                    className="computer-create-action primary"
                    onClick={handleComputerCreateStart}
                  >
                    {copy.noComputerStart}
                  </button>
                </>
              ) : computerCreateFlow.phase === 'command' ? (
                <>
                  {computerCreateFlow.machineName ? (
                    <p className="computer-create-machine-name">
                      {computerCreateFlow.machineName}
                    </p>
                  ) : null}
                  <code className="computer-create-command">
                    {computerCreateFlow.displayCommand}
                  </code>
                  <div className="computer-create-actions">
                    <button
                      type="button"
                      className="computer-create-action"
                      onClick={() => {
                        if (computerCreateFlow.daemonCommand) {
                          navigator.clipboard.writeText(
                            computerCreateFlow.daemonCommand,
                          )
                        }
                      }}
                    >
                      {copy.noComputerCopy}
                    </button>
                    <button
                      type="button"
                      className="computer-create-action primary"
                      onClick={handleComputerCreateStart}
                    >
                      {copy.noComputerExecute}
                    </button>
                  </div>
                </>
              ) : null}
            </div>
          </section>
        </section>
      ) : null}
    </main>
  )
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
  return {
    '--canvas': theme.canvas,
    '--toolbar': theme.toolbar,
    '--sidebar': theme.sidebar,
    '--panel': theme.panel,
    '--surface': theme.surface,
    '--surface-strong': theme.surfaceStrong,
    '--surface-secondary': theme.surfaceSecondary,
    '--surface-tertiary': theme.surfaceTertiary,
    '--line': theme.line,
    '--line-strong': theme.lineStrong,
    '--text': theme.text,
    '--muted': theme.muted,
    '--tertiary': theme.tertiary,
    '--danger': theme.danger,
    '--selection': theme.selection,
    '--hover': theme.hover,
    '--focus-ring': theme.focusRing,
    '--accent': theme.accent,
    '--accent-soft': theme.accentSoft,
    '--accent-hover': `color-mix(in srgb, ${theme.accent} 88%, black)`,
    '--accent-active': `color-mix(in srgb, ${theme.accent} 76%, black)`,
    '--on-accent': '#fff',
    '--radius-xs': theme.radiusXs + 'px',
    '--radius-sm': theme.radiusSm + 'px',
    '--radius-md': theme.radiusMd + 'px',
    '--radius-lg': theme.radiusLg + 'px',
    '--radius-xl': theme.radiusXl + 'px',
    '--radius-pill': theme.radiusPill + 'px',
    '--font-family': theme.fontFamily,
    '--font-family-mono': theme.fontFamilyMono,
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

function BotIcon() {
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
      <path d="M12 8V4H8" />
      <rect width="16" height="12" x="4" y="8" rx="2" />
      <path d="M2 14h2" />
      <path d="M20 14h2" />
      <path d="M15 13v2" />
      <path d="M9 13v2" />
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

function ChevronIcon({ direction }: { direction: 'up' | 'down' | 'left' | 'right' }) {
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
      {direction === 'up' ? <path d="m18 15-6-6-6 6" /> : direction === 'left' ? <path d="m15 18-6-6 6-6" /> : direction === 'right' ? <path d="m9 18 6-6-6-6" /> : <path d="m6 9 6 6 6-6" />}
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

// --- Shared ComposeBox component (#67 + #68) ---

function ComposeBox({
  text,
  setText,
  placeholder,
  sending,
  onSend,
  attachments,
  onFileSelect,
  onRemoveAttachment,
  disabled,
}: {
  text: string
  setText: (v: string) => void
  placeholder: string
  sending: boolean
  onSend: () => void
  attachments: { file: File; uploading: boolean; error?: boolean; id?: string }[]
  onFileSelect: (files: FileList) => void
  onRemoveAttachment: (file: File) => void
  disabled?: boolean
}) {
  const fileInputRef = useRef<HTMLInputElement>(null)
  const hasUploading = attachments.some((a) => a.uploading)
  const hasContent = text.trim().length > 0 || attachments.some((a) => a.id)
  const canSend = hasContent && !hasUploading

  return (
    <div className="compose-box">
      <textarea
        className="compose-textarea"
        placeholder={placeholder}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          // Skip Enter during IME composition (e.g. Chinese/Japanese input)
          if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing && e.keyCode !== 229) {
            e.preventDefault()
            if (canSend && !sending && !disabled) onSend()
          }
        }}
        disabled={sending || disabled}
        rows={3}
      />
      {attachments.length > 0 && (
        <div className="compose-attachments">
          {attachments.map((a, i) => (
            <div key={i} className={`compose-attachment-chip${a.error ? ' error' : ''}`}>
              <span className="compose-attachment-name">{a.file.name}</span>
              {a.uploading && <span className="compose-attachment-uploading">...</span>}
              {a.error && <span className="compose-attachment-error">failed</span>}
              <button
                type="button"
                className="compose-attachment-remove"
                onClick={() => onRemoveAttachment(a.file)}
                aria-label="Remove"
              >×</button>
            </div>
          ))}
        </div>
      )}
      <div className="compose-toolbar">
        <div className="compose-toolbar-left">
          <button
            type="button"
            className="compose-tool-button"
            title="Attach file"
            onClick={() => fileInputRef.current?.click()}
            disabled={sending || disabled}
          >📎</button>
          <button
            type="button"
            className="compose-tool-button"
            title="Insert image"
            onClick={() => fileInputRef.current?.click()}
            disabled={sending || disabled}
          >🖼</button>
          <input
            ref={fileInputRef}
            type="file"
            multiple
            className="sr-only"
            onChange={(e) => {
              if (e.target.files && e.target.files.length > 0) {
                onFileSelect(e.target.files)
              }
              e.target.value = ''
            }}
          />
        </div>
        <button
          type="button"
          className="compose-send-button"
          onClick={onSend}
          disabled={!canSend || sending || disabled}
        >
          {sending ? '...' : hasUploading ? 'Uploading...' : 'Send \u27A4'}
        </button>
      </div>
    </div>
  )
}

// --- Message list normalization (sort oldest→newest, dedupe) ---

function normalizeMessages(messages: InboxMessage[]): InboxMessage[] {
  const seen = new Set<string>()
  const unique: InboxMessage[] = []
  for (const msg of messages) {
    if (!seen.has(msg.id)) {
      seen.add(msg.id)
      unique.push(msg)
    }
  }
  unique.sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
  return unique
}

// --- Simple Markdown renderer ---

function sanitizeHref(raw: string): string | null {
  const trimmed = raw.trim()
  // Only allow safe protocols — block javascript:, data:, vbscript: etc.
  if (/^(?:https?|mailto):/i.test(trimmed)) {
    // Escape double-quotes to prevent attribute breakout
    return trimmed.replace(/"/g, '&quot;')
  }
  return null
}

function renderMarkdown(text: string): string {
  // Escape HTML entities first
  let html = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')

  // Fenced code blocks (```...```) — must come before inline code
  html = html.replace(/```([^`]*?)```/gs, '<pre><code>$1</code></pre>')
  // Inline code
  html = html.replace(/`([^`\n]+)`/g, '<code>$1</code>')
  // Bold (**text** or __text__)
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
  html = html.replace(/__(.+?)__/g, '<strong>$1</strong>')
  // Italic (*text* or _text_)
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>')
  html = html.replace(/(?<!\w)_(.+?)_(?!\w)/g, '<em>$1</em>')
  // Strikethrough (~~text~~)
  html = html.replace(/~~(.+?)~~/g, '<del>$1</del>')
  // Links [text](url)
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label: string, href: string) => {
    const safe = sanitizeHref(href)
    if (safe) {
      return `<a href="${safe}" target="_blank" rel="noopener noreferrer">${label}</a>`
    }
    return label
  })
  // Bare URLs (http/https)
  html = html.replace(/(?<!")(?<!=)\b(https?:\/\/[^\s<)]+)/g, (_match, url: string) => {
    const safe = sanitizeHref(url)
    if (safe) {
      return `<a href="${safe}" target="_blank" rel="noopener noreferrer">${url}</a>`
    }
    return url
  })
  // Line breaks
  html = html.replace(/\n/g, '<br/>')

  return html
}

// --- Relative time helper ---

function relativeTime(dateStr: string): string {
  const now = Date.now()
  const then = new Date(dateStr).getTime()
  const diff = now - then
  if (diff < 60_000) return 'just now'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`
  return new Date(dateStr).toLocaleDateString()
}

// --- Attachment preview in messages ---

function MessageAttachmentView({ att, serverUrl }: { att: MessageAttachment; serverUrl: string }) {
  const isImage = att.contentType?.startsWith('image/') ?? false
  const previewUrl = `${serverUrl}/api/attachments/${att.id}/preview`
  const downloadUrl = `${serverUrl}/api/attachments/${att.id}/url`

  if (isImage) {
    return (
      <div className="msg-attachment-image">
        <img src={previewUrl} alt={att.filename} loading="lazy" />
      </div>
    )
  }

  return (
    <a className="msg-attachment-file" href={downloadUrl} target="_blank" rel="noopener noreferrer">
      <span className="msg-attachment-icon">📎</span>
      <span className="msg-attachment-info">
        <span className="msg-attachment-filename">{att.filename}</span>
        {att.size != null && <span className="msg-attachment-size">{formatFileSize(att.size)}</span>}
      </span>
    </a>
  )
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1048576).toFixed(1)} MB`
}

function InboxSkeleton() {
  const items = [3, 2] // rows per skeleton group
  return (
    <>
      {items.map((count, gi) => (
        <div key={gi} className="inbox-skeleton-group">
          <div className="inbox-skeleton-header">
            <span className="inbox-skeleton-bar" />
          </div>
          {Array.from({ length: count }, (_, i) => (
            <div key={i} className="inbox-skeleton-item">
              <div className="inbox-skeleton-row">
                <span className="inbox-skeleton-bar source" />
                <span className="inbox-skeleton-bar time" />
              </div>
              <span className="inbox-skeleton-bar preview" />
            </div>
          ))}
        </div>
      ))}
    </>
  )
}

function EnterIcon() {
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
      <path d="m9 18 6-6-6-6" />
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

function createNewThemeDraft(accent = DEFAULT_NEW_THEME_ACCENT): NewThemeDraft {
  return {
    name: '',
    accent: normalizeHexColor(accent) ?? DEFAULT_NEW_THEME_ACCENT,
  }
}

function syncNewThemeDraftAccent(
  draft: NewThemeDraft,
  accent: string,
): NewThemeDraft {
  return {
    ...draft,
    accent: normalizeHexColor(accent) ?? draft.accent,
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
