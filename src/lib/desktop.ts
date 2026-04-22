import { invoke } from '@tauri-apps/api/core'

export interface ThemeDefinition {
  id: string
  name: string
  summary: string
  mode: 'light' | 'dark'
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

export interface BootstrapPayload {
  appName: string
  workspaceUrl: string
  activeThemeId: string
  workspaceOpen: boolean
  themes: ThemeDefinition[]
}

export async function loadBootstrap() {
  return invoke<BootstrapPayload>('bootstrap')
}

export async function updateTheme(themeId: string) {
  return invoke<BootstrapPayload>('set_theme', { themeId })
}

export async function openWorkspace() {
  return invoke<BootstrapPayload>('open_workspace')
}
