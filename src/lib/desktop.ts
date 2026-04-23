import { invoke } from '@tauri-apps/api/core'

export interface ThemeDefinition {
  id: string
  name: string
  summary: string
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
  name: string
  accent: string
}

export interface ServiceSnapshot {
  commandPath: string
  workingDirectory: string
  args: string[]
  autoStartWithWorkspace: boolean
  configured: boolean
  running: boolean
  pid: number | null
  lastError: string | null
}

export interface UpdateSnapshot {
  currentVersion: string
  repositorySlug: string
  releasesUrl: string
  latestReleaseApiUrl: string
}

export interface BootstrapPayload {
  appName: string
  workspaceUrl: string
  colorScheme: string
  appearanceMode: 'light' | 'dark' | 'system'
  customTheme: CustomThemeSnapshot
  language: 'en-US' | 'zh-CN' | 'system'
  resolvedLanguage: 'en-US' | 'zh-CN'
  workspaceOpen: boolean
  themes: ThemeDefinition[]
  service: ServiceSnapshot
  updates: UpdateSnapshot
}

export async function loadBootstrap() {
  return invoke<BootstrapPayload>('bootstrap')
}

export async function updateTheme(themeId: string) {
  return invoke<BootstrapPayload>('set_theme', { themeId })
}

export async function updateThemeMode(themeMode: BootstrapPayload['appearanceMode']) {
  return invoke<BootstrapPayload>('set_theme_mode', { themeMode })
}

export async function saveCustomTheme(customTheme: CustomThemeSnapshot) {
  return invoke<BootstrapPayload>('save_custom_theme', { customTheme })
}

export async function updateLanguage(language: BootstrapPayload['language']) {
  return invoke<BootstrapPayload>('set_language', { language })
}

export async function openWorkspace() {
  return invoke<BootstrapPayload>('open_workspace')
}

export async function saveServiceSettings(service: ServiceSnapshot) {
  return invoke<BootstrapPayload>('save_service_settings', {
    service: {
      commandPath: service.commandPath,
      workingDirectory: service.workingDirectory,
      args: service.args,
      autoStartWithWorkspace: service.autoStartWithWorkspace,
    },
  })
}

export async function startService() {
  return invoke<BootstrapPayload>('start_service')
}

export async function stopService() {
  return invoke<BootstrapPayload>('stop_service')
}

export async function saveUpdateSettings(updates: UpdateSnapshot) {
  return invoke<BootstrapPayload>('save_update_settings', {
    updates: {
      repositorySlug: updates.repositorySlug,
      releasesUrl: updates.releasesUrl,
    },
  })
}

export async function openExternalUrl(url: string) {
  return invoke('open_external_url', { url })
}
