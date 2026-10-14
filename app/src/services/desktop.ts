import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import type {
  AppPaths,
  AppSettings,
  BackgroundImageInfo,
  ComputeBackend,
  EnvironmentDiagnostics,
  EnvironmentInstallEvent,
  GenerationJob,
  JobPreview,
  ManagedPackage,
  ModelInfo,
  OsuInstallation,
  OsuImportLaunch,
  RuntimeInfo,
  SettingsImportPreview,
  StorageCategory,
  StorageCleanupPreview,
  StorageCleanupResult,
  StorageUsageReport,
  StartJobInput,
  SystemInfo,
  WorkerHealth,
} from '@/types'

export function isDesktopRuntime() {
  return '__TAURI_INTERNALS__' in window
}

export async function selectAudioFile(): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'm4a', 'flac'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function selectBeatmapFile(): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'osu! beatmap', extensions: ['osu'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function selectBackgroundImage(): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Background image', extensions: ['png', 'jpg', 'jpeg'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function authorizeBackgroundPreview(path: string) {
  const info = await invoke<BackgroundImageInfo>('authorize_background_preview', { path })
  return { ...info, url: convertFileSrc(info.path) }
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

export async function selectSettingsImportFile(): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Beatforge settings', extensions: ['json'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function selectSettingsExportFile(): Promise<string | null> {
  if (!isDesktopRuntime()) return null
  const selected = await save({
    defaultPath: 'beatforge-settings.json',
    filters: [{ name: 'Beatforge settings', extensions: ['json'] }],
  })
  return typeof selected === 'string' ? selected : null
}

export async function exportDesktopSettings(path: string, settings: AppSettings) {
  return invoke<string>('export_settings', { path, settings })
}

export async function previewDesktopSettingsImport(path: string) {
  return invoke<SettingsImportPreview>('preview_settings_import', { path })
}

export async function resetDesktopSettings(field?: keyof AppSettings) {
  return invoke<AppSettings>('reset_settings', { field: field ?? null })
}

export async function getStorageUsage() {
  return invoke<StorageUsageReport>('get_storage_usage')
}

export async function previewStorageCleanup(categories: StorageCategory[]) {
  return invoke<StorageCleanupPreview>('preview_storage_cleanup', { categories })
}

export async function cleanupStorage(categories: StorageCategory[], allowProtected: boolean) {
  return invoke<StorageCleanupResult>('cleanup_storage', { categories, allowProtected })
}

export async function createDesktopJob(input: StartJobInput) {
  return invoke<GenerationJob>('create_job', { input })
}

export async function listDesktopJobs() {
  return invoke<GenerationJob[]>('list_jobs')
}

export async function listManagedPackages() {
  return invoke<ManagedPackage[]>('list_managed_packages')
}

export async function downloadManagedPackage(packageId: string) {
  return invoke<ManagedPackage>('download_managed_package', { packageId })
}

export async function installPythonRuntime() {
  return invoke<ManagedPackage>('install_python_runtime')
}

export async function verifyPythonRuntime() {
  return invoke<RuntimeInfo>('verify_python_runtime')
}

export async function removePythonRuntime() {
  return invoke<ManagedPackage>('remove_python_runtime')
}

export async function installModelPackage(packageId: string) {
  return invoke<ManagedPackage>('install_model_package', { packageId })
}

export async function verifyModelPackage(packageId: string) {
  return invoke<ModelInfo>('verify_model_package', { packageId })
}

export async function removeModelPackage(packageId: string) {
  return invoke<ManagedPackage>('remove_model_package', { packageId })
}

export async function diagnoseAiEnvironment() {
  return invoke<EnvironmentDiagnostics>('diagnose_ai_environment')
}

export async function diagnoseProductionWorker() {
  return invoke<WorkerHealth>('diagnose_production_worker')
}

export async function installAiDependencies(backend: ComputeBackend) {
  return invoke<EnvironmentDiagnostics>('install_ai_dependencies', { backend })
}

export function onEnvironmentInstallEvent(
  handler: (event: EnvironmentInstallEvent) => void,
): Promise<UnlistenFn> {
  return listen<EnvironmentInstallEvent>('environment-install-event', (event) =>
    handler(event.payload),
  )
}

export function onPackageProgressEvent(
  handler: (
    event: Pick<ManagedPackage, 'id' | 'status' | 'progress' | 'bytesDownloaded' | 'sizeBytes'>,
  ) => void,
): Promise<UnlistenFn> {
  return listen('package-progress-event', (event) => {
    const payload = event.payload as {
      packageId: string
      status: ManagedPackage['status']
      progress: number
      bytesDownloaded: number
      sizeBytes: number
    }
    handler({ ...payload, id: payload.packageId })
  })
}

export async function cancelDesktopJob(jobId: string) {
  return invoke<void>('cancel_job', { jobId })
}

export async function retryDesktopJob(jobId: string) {
  return invoke<void>('retry_job', { jobId })
}

export async function regenerateDesktopJob(jobId: string) {
  return invoke<GenerationJob>('regenerate_job', { jobId })
}

export async function deleteDesktopJob(jobId: string) {
  return invoke<void>('delete_job', { jobId })
}

export async function revealDesktopJobOutput(jobId: string) {
  return invoke<void>('reveal_job_output', { jobId })
}

export async function importDesktopJobOutput(jobId: string) {
  return invoke<OsuImportLaunch>('import_job_output', { jobId })
}

export async function importFailedDesktopJobOutput(jobId: string) {
  return invoke<OsuImportLaunch>('import_failed_job_output', { jobId })
}

export async function importUnfilteredDesktopSetOutput(jobId: string) {
  return invoke<OsuImportLaunch>('import_unfiltered_set_output', { jobId })
}

export async function getDesktopJobPreview(jobId: string) {
  const preview = await invoke<JobPreview>('get_job_preview', { jobId })
  return {
    ...preview,
    audioUrl: preview.audioPath ? convertFileSrc(preview.audioPath) : null,
  }
}

export function onDesktopJobEvent(handler: (job: GenerationJob) => void): Promise<UnlistenFn> {
  return listen<GenerationJob>('job-event', (event) => handler(event.payload))
}
