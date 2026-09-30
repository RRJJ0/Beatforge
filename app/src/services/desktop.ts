import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  AppPaths,
  AppSettings,
  GenerationJob,
  OsuInstallation,
  StartJobInput,
  SystemInfo,
} from '@/types'

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

export async function createDesktopJob(input: StartJobInput) {
  return invoke<GenerationJob>('create_job', { input })
}

export async function listDesktopJobs() {
  return invoke<GenerationJob[]>('list_jobs')
}

export async function cancelDesktopJob(jobId: string) {
  return invoke<void>('cancel_job', { jobId })
}

export async function retryDesktopJob(jobId: string) {
  return invoke<void>('retry_job', { jobId })
}

export async function deleteDesktopJob(jobId: string) {
  return invoke<void>('delete_job', { jobId })
}

export function onDesktopJobEvent(handler: (job: GenerationJob) => void): Promise<UnlistenFn> {
  return listen<GenerationJob>('job-event', (event) => handler(event.payload))
}
