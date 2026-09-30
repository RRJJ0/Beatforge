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
export interface ModelPackage {
  id: string
  name: string
  version: string
  description: string
  size: string
  status: 'installed' | 'available' | 'downloading'
  progress?: number
  recommended?: boolean
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
