import { describe, expect, it } from 'vitest'
import type { GenerationJob } from '@/types'
import {
  TerminalJobTracker,
  buildJobNotice,
  shouldSendSystemNotification,
} from '../jobNotifications'

function job(status: GenerationJob['status']): GenerationJob {
  return {
    id: 'BF-TEST',
    title: 'Song',
    artist: 'Artist',
    audioFile: 'C:\\private\\song.flac',
    model: 'mapperatorinator-v32-standard',
    difficulty: 5,
    status,
    progress: 100,
    stage: 'C:\\private\\traceback.txt',
    scenario: 'success',
    createdAt: 'now',
    errorMessage: 'traceback C:\\private\\worker.py',
  }
}

describe('job notifications', () => {
  it('uses a minimal summary without local paths or traceback details', () => {
    const notice = buildJobNotice(job('failed'))!
    expect(notice.title).toBe('Beatmap generation failed')
    expect(notice.body).toContain('Artist — Song')
    expect(notice.body).not.toContain('C:\\')
    expect(notice.body.toLowerCase()).not.toContain('traceback')
  })

  it('notifies each terminal transition once and rearms after retry', () => {
    const tracker = new TerminalJobTracker()
    expect(tracker.consume('running', job('completed'))).not.toBeNull()
    expect(tracker.consume('completed', job('completed'))).toBeNull()
    expect(tracker.consume('completed', job('running'))).toBeNull()
    expect(tracker.consume('running', job('completed'))).not.toBeNull()
  })

  it('does not notify cancellation or interruption', () => {
    expect(buildJobNotice(job('cancelled'))).toBeNull()
    expect(buildJobNotice(job('interrupted'))).toBeNull()
  })

  it('sends system notifications only when enabled and in the background', () => {
    expect(shouldSendSystemNotification(true, 'hidden', false)).toBe(true)
    expect(shouldSendSystemNotification(true, 'visible', false)).toBe(true)
    expect(shouldSendSystemNotification(true, 'visible', true)).toBe(false)
    expect(shouldSendSystemNotification(false, 'hidden', false)).toBe(false)
  })
})
