export type JobStatus = 'queued' | 'running' | 'completed' | 'failed'
export interface GenerationJob {
  id: string; title: string; artist: string; audioFile: string; model: string; difficulty: number
  status: JobStatus; progress: number; stage: string; createdAt: string; eta?: string
}
export interface ModelPackage {
  id: string; name: string; version: string; description: string; size: string
  status: 'installed' | 'available' | 'downloading'; progress?: number; recommended?: boolean
}

