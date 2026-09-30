import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { AppPaths, AppSettings, OsuInstallation, SystemInfo } from '@/types'

export function isDesktopRuntime() {
  return '__TAURI_INTERNALS__' in window
}

export async function selectAudioFile(): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'flac'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function selectDirectory(defaultPath?: string): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await open({ multiple: false, directory: true, defaultPath })
  return typeof selected === 'string' ? selected : null
}

export async function getAppPaths() {
  return invoke<AppPaths>('get_app_paths')
}

export async function getSystemInfo() {
  return invoke<SystemInfo>('get_system_info')
}

export async function detectOsuInstallations() {
  return invoke<OsuInstallation[]>('detect_osu_installations')
}

export async function loadDesktopSettings() {
  return invoke<AppSettings>('load_settings')
}

export async function saveDesktopSettings(settings: AppSettings) {
  return invoke<void>('save_settings', { settings })
}
