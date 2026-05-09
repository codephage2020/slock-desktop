import { invoke } from '@tauri-apps/api/core'

export interface ThemeDefinition {
  id: string
  name: string
  summary: string
  styleId: string
  styleName: string
  mode: 'light' | 'dark' | 'system'
  canvas: string
  surface: string
  surfaceStrong: string
  line: string
  text: string
  muted: string
  accent: string
  accentSoft: string
  preview: [string, string, string]
}

export interface CustomThemeSnapshot {
  id: string
  name: string
  accent: string
}

export interface ThemeStylePalette {
  canvas: string
  toolbar: string
  sidebar: string
  panel: string
  surface: string
  surfaceStrong: string
  surfaceSecondary: string
  surfaceTertiary: string
  line: string
  lineStrong: string
  text: string
  muted: string
  tertiary: string
  danger: string
  selection: string
  hover: string
  focusRing: string
}

export interface ThemeStyleConfig {
  id: string
  name: string
  summary: string
  native: boolean
  light: ThemeStylePalette
  dark: ThemeStylePalette
  accentSoftLightMix: number
  accentSoftDarkMix: number
  radiusXs: number
  radiusSm: number
  radiusMd: number
  radiusLg: number
  radiusXl: number
  radiusPill: number
  fontFamily: string
  fontFamilyMono: string
}

export interface ThemeStyleDefinition {
  id: string
  name: string
  summary: string
  preview: [string, string, string]
  builtIn: boolean
  config: ThemeStyleConfig
}

export interface ServiceSnapshot {
  serverUrl: string
  selectedServerSlug: string
  activeServerSlug: string
  autoStartWithWorkspace: boolean
  closeAppBehavior: 'ask' | 'keep' | 'stop'
  authenticated: boolean
  account: ServiceAccountSnapshot | null
  accounts: ServiceAccountSnapshot[]
  configured: boolean
  running: boolean
  pid: number | null
  lastError: string | null
  syncError: string | null
  servers: ServiceServerSnapshot[]
}

export interface ServiceAccountSnapshot {
  id: string
  displayName: string | null
  email: string | null
  avatarUrl: string | null
  initials: string
}

export interface ServiceServerSnapshot {
  id: string
  name: string
  slug: string
  selected: boolean
  machineId: string | null
  machineName: string | null
  machineStatus: string
  apiKeyReady: boolean
  bindingSource: string
}

export interface UpdateSnapshot {
  currentVersion: string
  latest: DesktopUpdateCheck | null
}

export interface DesktopUpdateCheck {
  currentVersion: string
  available: boolean
  version: string | null
  body: string | null
  date: string | null
  downloadUrl: string | null
}

export interface ServiceLogSnapshot {
  serverSlug: string
  path: string
  content: string
  truncated: boolean
  totalBytes: number
  fromEpochMs: number
  toEpochMs: number
  timestampFiltered: boolean
}

export interface ServiceLogRange {
  fromEpochMs?: number | null
  toEpochMs?: number | null
}

export interface BootstrapPayload {
  appName: string
  workspaceUrl: string
  colorScheme: string
  styleScheme: string
  appearanceMode: 'light' | 'dark' | 'system'
  customThemes: CustomThemeSnapshot[]
  customStyles: ThemeStyleConfig[]
  language: 'en-US' | 'zh-CN' | 'system'
  resolvedLanguage: 'en-US' | 'zh-CN'
  workspaceOpen: boolean
  themes: ThemeDefinition[]
  themeStyles: ThemeStyleDefinition[]
  service: ServiceSnapshot
  updates: UpdateSnapshot
}

export async function loadBootstrap(refresh = true) {
  return invoke<BootstrapPayload>('bootstrap', { refresh })
}

export async function updateTheme(themeId: string) {
  return invoke<BootstrapPayload>('set_theme', { themeId })
}

export async function updateThemeStyle(styleId: string) {
  return invoke<BootstrapPayload>('set_theme_style', { styleId })
}

export async function importThemeStyle(config: ThemeStyleConfig) {
  return invoke<BootstrapPayload>('import_theme_style', { config })
}

export async function updateThemeMode(themeMode: BootstrapPayload['appearanceMode']) {
  return invoke<BootstrapPayload>('set_theme_mode', { themeMode })
}

export async function createCustomTheme(input: { name: string; accent: string }) {
  return invoke<BootstrapPayload>('create_custom_theme', input)
}

export async function renameCustomTheme(input: { id: string; name: string }) {
  return invoke<BootstrapPayload>('rename_custom_theme', input)
}

export async function updateCustomThemeAccent(input: { id: string; accent: string }) {
  return invoke<BootstrapPayload>('update_custom_theme_accent', input)
}

export async function deleteCustomTheme(input: { id: string }) {
  return invoke<BootstrapPayload>('delete_custom_theme', input)
}

export async function updateLanguage(language: BootstrapPayload['language']) {
  return invoke<BootstrapPayload>('set_language', { language })
}

export async function openWorkspace(selectedServerSlug?: string) {
  return invoke<BootstrapPayload>('open_workspace', { selectedServerSlug })
}

export async function exitWorkspace() {
  return invoke<BootstrapPayload>('exit_workspace')
}

export async function openLogin() {
  return invoke<BootstrapPayload>('open_login')
}

export async function openLoginBrowser() {
  return invoke<BootstrapPayload>('open_login_browser')
}

export async function switchAccount() {
  return invoke<BootstrapPayload>('switch_account')
}

export async function switchAccountBrowser() {
  return invoke<BootstrapPayload>('switch_account_browser')
}

export async function activateAccount(accountId: string) {
  return invoke<BootstrapPayload>('activate_account', { accountId })
}

export async function forgetAccount(accountId: string) {
  return invoke<BootstrapPayload>('forget_account', { accountId })
}

export async function selectServiceServer(selectedServerSlug: string) {
  return invoke<BootstrapPayload>('select_service_server', { selectedServerSlug })
}

export async function saveServiceSettings(service: ServiceSnapshot) {
  return invoke<BootstrapPayload>('save_service_settings', {
    service: {
      serverUrl: service.serverUrl,
      selectedServerSlug: service.selectedServerSlug,
      autoStartWithWorkspace: service.autoStartWithWorkspace,
      closeAppBehavior: service.closeAppBehavior,
    },
  })
}

export async function startService(selectedServerSlug?: string) {
  return invoke<BootstrapPayload>('start_service', { selectedServerSlug })
}

export async function stopService(selectedServerSlug?: string) {
  return invoke<BootstrapPayload>('stop_service', { selectedServerSlug })
}

export async function refreshServiceServers() {
  return invoke<BootstrapPayload>('refresh_service_servers')
}

export async function refreshServiceServerStatus() {
  return invoke<BootstrapPayload>('refresh_service_server_status')
}

export async function refreshServiceServerCatalog() {
  return invoke<BootstrapPayload>('refresh_service_server_catalog')
}

export async function updateService(selectedServerSlug?: string) {
  return invoke<BootstrapPayload>('update_service', { selectedServerSlug })
}

export async function openServiceLog(serverSlug: string, range: ServiceLogRange = {}) {
  return invoke<ServiceLogSnapshot>('open_service_log', { serverSlug, ...range })
}

export async function checkDesktopUpdate() {
  return invoke<DesktopUpdateCheck>('check_desktop_update')
}

export async function installDesktopUpdate() {
  return invoke('install_desktop_update')
}

// Dashboard types and commands

export interface DashboardChannel {
  id: string
  name: string
  type: string
  isArchived: boolean
  lastMessageAt: string | null
  memberCount: number
}

export interface DashboardChannelUnread {
  channelId: string
  unreadCount: number
}

export interface DashboardTask {
  id: string
  title: string
  status: 'todo' | 'in_progress' | 'in_review' | 'done'
  assignee: string | null
  channelId: string
}

export interface DashboardAgent {
  id: string
  name: string
  displayName: string | null
  description: string | null
  status: 'online' | 'offline' | 'running' | 'busy' | 'idle'
  updatedAt: string | null
}

export interface AgentActivityEntry {
  id: string
  activity: string
  detail: string | null
  launchId: string | null
  createdAt: string | null
  agentId?: string | null
  // Extra fields from API (catch-all via serde flatten)
  [key: string]: unknown
}

export interface DashboardData {
  channels: DashboardChannel[]
  unread: DashboardChannelUnread[]
  tasks: DashboardTask[]
  agents: DashboardAgent[]
  warnings?: string[]
}

export async function fetchDashboard(serverSlug: string) {
  return invoke<DashboardData>('fetch_dashboard', { serverSlug })
}

export async function fetchAgentActivity(serverSlug: string, agentId: string) {
  return invoke<AgentActivityEntry[]>('fetch_agent_activity', { serverSlug, agentId })
}

export async function stopAgent(serverSlug: string, agentId: string) {
  return invoke<void>('stop_agent', { serverSlug, agentId })
}

export async function startAgent(serverSlug: string, agentId: string) {
  return invoke<void>('start_agent', { serverSlug, agentId })
}

// Inbox types and commands

export interface InboxThread {
  id: string
  name: string | null
  parentChannelId: string | null
  parentChannelName: string | null
  parentMessageId: string | null
  isDone: boolean
  lastMessageAt: string | null
  unreadCount: number
}

export interface InboxDmMember {
  id: string
  name: string
  displayName: string | null
  avatarUrl: string | null
}

export interface InboxDmChannel {
  id: string
  name: string
  displayName: string | null
  lastMessageAt: string | null
  unreadCount: number
  members: InboxDmMember[]
}

export interface InboxMessage {
  id: string
  seq: number | null
  channelId: string
  content: string
  senderId: string | null
  senderName: string | null
  senderType: string | null
  senderDisplayName: string | null
  senderAvatarUrl: string | null
  createdAt: string
  updatedAt: string | null
}

export interface InboxMessagesResponse {
  messages: InboxMessage[]
  hasMore: boolean
}

export interface InboxUnreadEntry {
  channelId: string
  unreadCount: number
}

export async function fetchFollowedThreads(serverSlug: string) {
  return invoke<InboxThread[]>('fetch_followed_threads', { serverSlug })
}

export async function fetchDmChannels(serverSlug: string) {
  return invoke<InboxDmChannel[]>('fetch_dm_channels', { serverSlug })
}

export async function fetchUnreadChannels(serverSlug: string) {
  return invoke<InboxUnreadEntry[]>('fetch_unread_channels', { serverSlug })
}

export async function fetchThreadMessages(
  serverSlug: string,
  channelId: string,
  options?: { limit?: number; before?: string; after?: string }
) {
  return invoke<InboxMessagesResponse>('fetch_thread_messages', {
    serverSlug,
    channelId,
    limit: options?.limit,
    before: options?.before,
    after: options?.after,
  })
}

export async function sendMessage(serverSlug: string, channelId: string, content: string) {
  return invoke<InboxMessage>('send_message', { serverSlug, channelId, content })
}

export async function markChannelRead(serverSlug: string, channelId: string) {
  return invoke<void>('mark_channel_read', { serverSlug, channelId })
}

export async function bindLocalMachine(serverSlug: string, machineId: string) {
  return invoke<BootstrapPayload>('bind_local_machine', { serverSlug, machineId })
}

// Computer creation flow types and commands

export interface ServerMachinesCheck {
  hasMachines: boolean
  machineCount: number
  serverSlug: string
  createUrl: string
}

export async function checkServerMachines(serverSlug: string) {
  return invoke<ServerMachinesCheck>('check_server_machines', { serverSlug })
}

export async function openComputerCreatePage(serverSlug: string) {
  return invoke<void>('open_computer_create_page', { serverSlug })
}

// Unified inbox types and commands

export interface ServerMember {
  id: string
  name: string
  displayName: string | null
  avatarUrl: string | null
  role: string | null
}

export interface ServerUnreadEntry {
  serverId: string
  unreadCount: number
}

export async function fetchChannelMessages(
  serverSlug: string,
  channelId: string,
  options?: { limit?: number; before?: string; after?: string }
) {
  return invoke<InboxMessagesResponse>('fetch_channel_messages', {
    serverSlug,
    channelId,
    limit: options?.limit,
    before: options?.before,
    after: options?.after,
  })
}

export async function fetchServerMembers(serverSlug: string) {
  return invoke<ServerMember[]>('fetch_server_members', { serverSlug })
}

export async function fetchServerUnreadSummary() {
  return invoke<ServerUnreadEntry[]>('fetch_server_unread_summary')
}
