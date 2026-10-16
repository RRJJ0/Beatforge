import { describe, expect, it } from 'vitest'
import type { GenerationJob } from '@/types'
import { filterAndSortJobs, parseJobDate, type HistoryFilters } from '../jobHistory'

const jobs: GenerationJob[] = [
  {
    id: 'BF-3',
    title: 'Gamma',
    artist: 'Artist C',
    audioFile: 'gamma.flac',
    model: 'model-b',
    difficulty: 6,
    status: 'validation_failed',
    progress: 100,
    stage: 'Invalid',
    scenario: 'success',
    createdAt: '2026-10-13 02:00:00',
  },
  {
    id: 'BF-2',
    title: 'Beta',
    artist: 'Artist B',
    audioFile: 'beta.flac',
    model: 'model-a',
    difficulty: 5,
    status: 'completed',
    progress: 100,
    stage: 'Completed',
    scenario: 'success',
    createdAt: '2026-10-10 02:00:00',
  },
  {
    id: 'BF-1',
    title: 'Alpha',
    artist: 'Artist A',
    audioFile: 'alpha.flac',
    model: 'model-a',
    difficulty: 4,
    status: 'failed',
    progress: 30,
    stage: 'Failed',
    scenario: 'success',
    createdAt: '2026-09-01 02:00:00',
  },
]

const defaults: HistoryFilters = {
  search: '',
  status: 'all',
  model: 'all',
  dateRange: 'all',
  sort: 'newest',
}

describe('job history filtering', () => {
  it('parses SQLite UTC timestamps', () => {
    expect(parseJobDate('2026-10-13 02:00:00')?.toISOString()).toBe('2026-10-13T02:00:00.000Z')
  })

  it('combines keyword, status, model, and date filters', () => {
    const result = filterAndSortJobs(
      jobs,
      {
        ...defaults,
        search: 'artist b',
        status: 'completed',
        model: 'model-a',
        dateRange: 'seven_days',
      },
      new Date('2026-10-13T12:00:00Z'),
    )
    expect(result.map((job) => job.id)).toEqual(['BF-2'])
  })

  it('sorts deterministically by oldest date or title', () => {
    expect(filterAndSortJobs(jobs, { ...defaults, sort: 'oldest' }).map((job) => job.id)).toEqual([
      'BF-1',
      'BF-2',
      'BF-3',
    ])
    expect(filterAndSortJobs(jobs, { ...defaults, sort: 'title' }).map((job) => job.id)).toEqual([
      'BF-1',
      'BF-2',
      'BF-3',
    ])
  })
})
