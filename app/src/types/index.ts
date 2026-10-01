export type JobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled' | 'interrupted'
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
}

export interface StartJobInput {
  title: string
  artist: string
  audioFile: string
  model: string
  difficulty: number
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
