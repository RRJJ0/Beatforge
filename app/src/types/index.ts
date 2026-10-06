export type JobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled' | 'interrupted'
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

export interface GenerationJob {
  id: string
  title: string
  artist: string
  audioFile: string
  model: string
  difficulty: number
  status: JobStatus
  progress: number
  stage: string
  scenario: string
  createdAt: string
  eta?: string
  errorMessage?: string
  outputPath?: string
  provenance?: GenerationProvenance | null
}

export interface StartJobInput {
  title: string
  artist: string
  audioFile: string
  model: string
  difficulty: number
  gamemode: number
  seed: number
  descriptors: string[]
  negativeDescriptors: string[]
  referenceBeatmap?: string
  approachRate?: number
  circleSize?: number
  hitsounded: boolean
  scenario: string
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
  checkUpdates: boolean
}
