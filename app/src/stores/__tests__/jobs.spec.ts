import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useJobsStore } from '../jobs'

const storage = new Map<string, string>()
vi.stubGlobal('localStorage', {
  clear: () => storage.clear(),
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => storage.set(key, value),
  removeItem: (key: string) => storage.delete(key),
})

describe('jobs store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('adds and persists a generation job', () => {
    const store = useJobsStore()
    const initialCount = store.jobs.length
    store.addJob({
      title: 'Test Track',
      artist: 'Test Artist',
      audioFile: 'track.mp3',
      model: 'mapperatorinator-v32-standard',
      difficulty: 4.5,
      gamemode: 0,
      seed: 12345,
      descriptors: [],
      negativeDescriptors: [],
      hitsounded: true,
      scenario: 'success',
    })
    expect(store.jobs).toHaveLength(initialCount + 1)
    expect(store.jobs[0]?.status).toBe('queued')
    expect(localStorage.getItem('beatforge:jobs')).toContain('Test Track')
  })

  it('removes a job by id', async () => {
    const store = useJobsStore()
    const id = store.jobs[0]!.id
    await store.removeJob(id)
    expect(store.jobs.some((job) => job.id === id)).toBe(false)
  })

  it('does not count a completed history record whose result file is missing', () => {
    const store = useJobsStore()
    store.jobs.unshift({
      id: 'BF-MISSING',
      title: 'Missing result',
      artist: 'Test Artist',
      audioFile: 'track.mp3',
      model: 'mapperatorinator-v32-standard',
      difficulty: 4.5,
      status: 'completed',
      progress: 100,
      stage: 'Completed and validated',
      scenario: 'success',
      createdAt: '2026-10-13T00:00:00Z',
      outputPath: 'C:\\missing\\result.osz',
      outputAvailable: false,
    })

    expect(store.completedJobs.some((job) => job.id === 'BF-MISSING')).toBe(false)
  })
})
