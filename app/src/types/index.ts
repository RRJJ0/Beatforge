export type JobStatus =
  | 'queued'
  | 'running'
  | 'validating'
  | 'completed'
  | 'validation_failed'
  | 'failed'
  | 'cancelled'
  | 'interrupted'
export interface GenerationProvenance {
  schema_version: number
  job_id: string
  reproduction_key: string
  source: {
    mapperatorinator_commit: string
    worker_version: string
    protocol_version: number
  }
  model: { package: string; version: string }
  runtime: {
    python_version: string
    pytorch_version: string
    cuda_runtime?: string
    device: string
    precision: string
    gpu_name?: string
    gpu_memory_total_mb?: number
    driver_version?: string
    peak_gpu_memory_mb?: number
  }
  parameters: Record<string, unknown>
  input: Record<string, unknown>
  output: Record<string, unknown>
  timing: { started_at: string; completed_at: string; elapsed_seconds: number }
  known_limitations: string[]
  difficulties?: Array<{
    id: string
    name: string
    status: 'completed' | 'validated' | 'failed' | 'validation_failed'
    error?: string
    provenance?: GenerationProvenance
    statistics?: BeatmapStatistics
  }>
}

export type ValidationStatus = 'valid' | 'valid_with_warnings' | 'repaired' | 'invalid'
export type ValidationSeverity = 'error' | 'warning' | 'info'

export interface ValidationIssue {
  code: string
  severity: ValidationSeverity
  section?: string | null
  line?: number | null
  message: string
  repairable: boolean
}

export interface BeatmapStatistics {
  beatmapCount: number
  circleCount: number
  sliderCount: number
  spinnerCount: number
  maxCombo?: number | null
  starRating?: number | null
  bpmMin?: number | null
  bpmMax?: number | null
  durationMs?: number | null
}

export interface RepairRecord {
  code: string
  section?: string | null
  field?: string | null
  before?: string | null
  after?: string | null
  reason: string
}

export interface ValidationReport {
  schemaVersion: number
  validatorVersion: string
  jobId: string
  status: ValidationStatus
  sourcePath: string
  validatedPath?: string | null
  issues: ValidationIssue[]
  statistics?: BeatmapStatistics | null
  repairs: RepairRecord[]
  startedAt: string
  completedAt: string
}

export interface DifficultyPreview {
  id: string
  name: string
  objectCount: number
  firstObjectMs?: number | null
  lastObjectMs?: number | null
  binWidthMs: number
  density: number[]
}

export interface JobPreview {
  jobId: string
  audioPath?: string | null
  audioUnavailableReason?: string | null
  waveformAllowed: boolean
  maxWaveformPoints: number
  difficulties: DifficultyPreview[]
}

export interface GenerationJob {
  id: string
  title: string
  artist: string
  audioFile: string
  model: string
  difficulty: number
  difficultyCount?: number
  status: JobStatus
  progress: number
  stage: string
  scenario: string
  createdAt: string
  eta?: string
  errorMessage?: string
  outputPath?: string
  outputAvailable?: boolean
  provenance?: GenerationProvenance | null
  validationReport?: ValidationReport | null
}

export interface DifficultyRequest {
  id: string
  name: string
  difficulty: number
  seed: number
  descriptors: string[]
  negativeDescriptors: string[]
  approachRate?: number
  circleSize?: number
  hitsounded: boolean
}

export interface StartJobInput {
  title: string
  artist: string
  audioFile: string
  model: string
  difficulty: number
  difficulties?: DifficultyRequest[]
  gamemode: number
  seed: number
  descriptors: string[]
  negativeDescriptors: string[]
  referenceBeatmap?: string
  backgroundImage?: string
  videoFile?: string
  videoStartTimeMs?: number
  approachRate?: number
  circleSize?: number
  hitsounded: boolean
  scenario: string
}

export interface BackgroundImageInfo {
  path: string
  width: number
  height: number
  sizeBytes: number
  format: string
}
export interface VideoInfo {
  path: string
  sizeBytes: number
  format: string
}
export type PackageKind = 'runtime' | 'model'
export type PackageStatus =
  | 'not_installed'
  | 'queued'
  | 'downloading'
  | 'verifying'
  | 'downloaded'
  | 'installing'
  | 'installed'
  | 'update_available'
  | 'removing'
  | 'failed'

export interface ManagedPackage {
  id: string
  kind: PackageKind
  name: string
  description: string
  targetVersion: string
  installedVersion?: string
  status: PackageStatus
  progress: number
  bytesDownloaded: number
  sizeBytes: number
  artifactPath?: string
  installPath?: string
  errorMessage?: string
  recommended: boolean
  updatedAt: string
}

export interface RuntimeInfo {
  version: string
  path: string
  executable: string
  healthy: boolean
}

export interface ModelInfo {
  id: string
  version: string
  path: string
  sizeBytes: number
  files: string[]
  healthy: boolean
}

export type ComputeBackend = 'cuda130' | 'cuda128' | 'cuda126' | 'cpu'

export interface ComponentDiagnostic {
  installed: boolean
  version?: string
  path?: string
  error?: string
}

export interface EnvironmentDiagnostics {
  python: ComponentDiagnostic
  pip: ComponentDiagnostic
  ffmpeg: ComponentDiagnostic
  pytorch: ComponentDiagnostic
  mapperatorinator: ComponentDiagnostic
  inferenceDependencies: ComponentDiagnostic
  gpuName?: string
  driverVersion?: string
  cudaAvailable: boolean
  recommendedBackend: ComputeBackend
  compatibilityReason: string
  ready: boolean
}

export interface EnvironmentInstallEvent {
  stage: string
  progress: number
  message: string
}

export interface WorkerHealth {
  protocolVersion: number
  workerVersion: string
  ready: boolean
  pythonVersion: string
  pythonExecutable: string
  pytorchVersion?: string
  cudaRuntime?: string
  cudaAvailable: boolean
  gpuName?: string
  ffmpegVersion?: string
  ffmpegExecutable?: string
  mapperatorinatorCommit?: string
  modelVersion?: string
  modelPath?: string
  errors: string[]
}

export interface AppPaths {
  root: string
  models: string
  runtime: string
  cache: string
  logs: string
  jobs: string
  outputs: string
  database: string
}

export interface GpuInfo {
  name: string
  memoryTotalMb?: number
  memoryFreeMb?: number
  driverVersion?: string
  available: boolean
}

export interface SystemInfo {
  osName: string
  osVersion: string
  architecture: string
  cpuName: string
  cpuCores: number
  totalMemoryMb: number
  availableMemoryMb: number
  availableDiskGb: number
  gpu: GpuInfo
}

export interface OsuInstallation {
  kind: string
  path: string
  songsPath?: string
  executable?: string
}

export interface OsuImportLaunch {
  method: 'executable' | 'file_association'
  application?: string | null
  artifact: string
}

export interface AppSettings {
  outputPath: string
  tempPath: string
  osuPath: string
  device: string
  precision: string
  workers: number
  autoImport: boolean
  keepIntermediate: boolean
  repairGeneratedOutput: boolean
  checkUpdates: boolean
  desktopNotifications: boolean
  locale: AppLocale
}

export interface AppUpdateCheck {
  currentVersion: string
  availableVersion?: string
  status: 'available' | 'up_to_date' | 'offline'
  notes?: string
  publishedAt?: string
  message?: string
}

export type AppLocale = 'en' | 'zh-TW'

export interface SettingsImportItem {
  field: keyof AppSettings
  current: unknown
  imported?: unknown
  result: unknown
  action: 'apply' | 'ignored' | 'reset' | 'unchanged'
  reason: string
}

export interface SettingsImportPreview {
  schemaVersion: number
  sourcePath: string
  items: SettingsImportItem[]
  resultingSettings: AppSettings
}

export interface DiagnosticFilePreview {
  name: string
  description: string
  sizeBytes: number
}

export interface DiagnosticPreview {
  schemaVersion: number
  diagnosticId: string
  files: DiagnosticFilePreview[]
  excluded: string[]
  privacyNotice: string
  totalBytes: number
}

export interface DiagnosticExportResult {
  path: string
  diagnosticId: string
  files: number
}

export type StorageCategory =
  'runtime' | 'models' | 'cache' | 'logs' | 'jobsRaw' | 'jobsWorking' | 'jobsValidated'

export interface StorageUsageCategory {
  category: StorageCategory
  bytes: number
  files: number
  directories: number
  latestModifiedMs?: number | null
  cleanable: boolean
  protected: boolean
  defaultSelected: boolean
  eligibleBytes: number
  eligibleFiles: number
  eligibleDirectories: number
  rejectedItems: number
}

export interface StorageUsageReport {
  scannedAtMs: number
  totalBytes: number
  activeJobs: string[]
  categories: StorageUsageCategory[]
  warnings: string[]
}

export interface StorageCleanupPreviewItem {
  category: StorageCategory
  bytes: number
  files: number
  directories: number
  targets: number
  protected: boolean
}

export interface StorageCleanupPreview {
  categories: StorageCleanupPreviewItem[]
  totalBytes: number
  totalFiles: number
  totalDirectories: number
  protectedSelected: boolean
  activeJobs: string[]
  warnings: string[]
}

export interface StorageCleanupFailure {
  category: StorageCategory
  target: string
  error: string
}

export interface StorageCleanupResult {
  freedBytes: number
  removedFiles: number
  removedDirectories: number
  failures: StorageCleanupFailure[]
  usage: StorageUsageReport
}
