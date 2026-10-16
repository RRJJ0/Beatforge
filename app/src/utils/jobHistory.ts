import type { GenerationJob, JobStatus } from '@/types'

export type HistoryDateRange = 'all' | 'today' | 'seven_days' | 'thirty_days'
export type HistorySort = 'newest' | 'oldest' | 'title'

export interface HistoryFilters {
  search: string
  status: 'all' | JobStatus
  model: 'all' | string
  dateRange: HistoryDateRange
  sort: HistorySort
}

export function parseJobDate(value: string): Date | null {
  const normalized = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(value)
    ? `${value.replace(' ', 'T')}Z`
    : value
  const date = new Date(normalized)
  return Number.isNaN(date.getTime()) ? null : date
}

function rangeStart(range: HistoryDateRange, now: Date) {
  if (range === 'all') return null
  const start = new Date(now)
  start.setHours(0, 0, 0, 0)
  if (range === 'seven_days') start.setDate(start.getDate() - 6)
  if (range === 'thirty_days') start.setDate(start.getDate() - 29)
  return start.getTime()
}

export function filterAndSortJobs(
  jobs: GenerationJob[],
  filters: HistoryFilters,
  now = new Date(),
) {
  const query = filters.search.trim().toLocaleLowerCase()
  const minimumTime = rangeStart(filters.dateRange, now)

  return jobs
    .filter((job) => {
      const matchesSearch =
        !query ||
        [job.title, job.artist, job.id, job.model, job.audioFile].some((value) =>
          value.toLocaleLowerCase().includes(query),
        )
      if (!matchesSearch || (filters.status !== 'all' && job.status !== filters.status)) {
        return false
      }
      if (filters.model !== 'all' && job.model !== filters.model) return false
      if (minimumTime !== null) {
        const created = parseJobDate(job.createdAt)
        if (!created || created.getTime() < minimumTime || created.getTime() > now.getTime()) {
          return false
        }
      }
      return true
    })
    .sort((left, right) => {
      if (filters.sort === 'title') {
        return (
          left.title.localeCompare(right.title, undefined, { sensitivity: 'base' }) ||
          left.id.localeCompare(right.id)
        )
      }
      const leftTime = parseJobDate(left.createdAt)?.getTime() ?? 0
      const rightTime = parseJobDate(right.createdAt)?.getTime() ?? 0
      return filters.sort === 'oldest' ? leftTime - rightTime : rightTime - leftTime
    })
}

export function formatJobDate(value: string) {
  const date = parseJobDate(value)
  if (!date) return value
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}
