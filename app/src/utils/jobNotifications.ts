import type { GenerationJob, JobStatus } from '@/types'

export interface JobNotice {
  jobId: string
  title: string
  body: string
  kind: 'success' | 'warning' | 'error'
}

const activeStatuses = new Set<JobStatus>(['queued', 'running', 'validating'])

export function buildJobNotice(job: GenerationJob): JobNotice | null {
  const song = `${job.artist} — ${job.title}`
  if (job.status === 'completed') {
    return {
      jobId: job.id,
      title: 'Beatmap generation complete',
      body: `${song} · ${job.difficulty.toFixed(1)}★ is ready.`,
      kind: 'success',
    }
  }
  if (job.status === 'validation_failed') {
    return {
      jobId: job.id,
      title: 'Beatmap needs review',
      body: `${song} did not pass output validation.`,
      kind: 'warning',
    }
  }
  if (job.status === 'failed') {
    return {
      jobId: job.id,
      title: 'Beatmap generation failed',
      body: `${song} could not be generated. Open Jobs for details.`,
      kind: 'error',
    }
  }
  return null
}

export class TerminalJobTracker {
  private notified = new Map<string, JobStatus>()

  consume(_previousStatus: JobStatus | undefined, job: GenerationJob): JobNotice | null {
    if (activeStatuses.has(job.status)) {
      this.notified.delete(job.id)
      return null
    }
    const notice = buildJobNotice(job)
    if (!notice || this.notified.get(job.id) === job.status) {
      return null
    }
    this.notified.set(job.id, job.status)
    return notice
  }
}

export function shouldSendSystemNotification(
  enabled: boolean,
  visibilityState: DocumentVisibilityState,
  hasFocus: boolean,
) {
  return enabled && (visibilityState !== 'visible' || !hasFocus)
}
