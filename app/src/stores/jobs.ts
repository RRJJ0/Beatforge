import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { GenerationJob, StartJobInput } from '@/types'
import { simulateProgress, type MockTask } from '@/services/mockBackend'
import {
  cancelDesktopJob,
  createDesktopJob,
  deleteDesktopJob,
  isDesktopRuntime,
  listDesktopJobs,
  onDesktopJobEvent,
  retryDesktopJob,
} from '@/services/desktop'

const seedJobs: GenerationJob[] = [
  {
    id: 'BF-DEMO-1',
    title: 'Daybreak',
    artist: 'Mizuki',
    audioFile: 'daybreak.flac',
    model: 'Mapperatorinator Standard',
    difficulty: 5.8,
    status: 'completed',
    progress: 100,
    stage: 'Browser prototype result',
    scenario: 'success',
    createdAt: 'Demo data',
  },
]

export const useJobsStore = defineStore('jobs', () => {
  const desktop = isDesktopRuntime()
  const saved = desktop ? null : localStorage.getItem('beatforge:jobs')
  const jobs = ref<GenerationJob[]>(saved ? JSON.parse(saved) : desktop ? [] : seedJobs)
  const initialized = ref(false)
  const loading = ref(false)
  const error = ref('')
  const mockTasks = new Map<string, MockTask>()

  const activeCount = computed(
    () =>
      jobs.value.filter((job) => ['running', 'queued', 'validating'].includes(job.status)).length,
  )
  const completedJobs = computed(() => jobs.value.filter((job) => job.status === 'completed'))
  const completedToday = computed(() => completedJobs.value.length)

  function upsert(job: GenerationJob) {
    const index = jobs.value.findIndex((item) => item.id === job.id)
    if (index === -1) jobs.value.unshift(job)
    else jobs.value[index] = job
  }

  async function initialize() {
    if (initialized.value) return
    initialized.value = true
    if (!desktop) return
    try {
      await onDesktopJobEvent(upsert)
      await refreshJobs()
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
    }
  }

  async function refreshJobs() {
    if (!desktop) return
    loading.value = true
    error.value = ''
    try {
      jobs.value = await listDesktopJobs()
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      throw reason
    } finally {
      loading.value = false
    }
  }

  function persistBrowserJobs() {
    if (!desktop) localStorage.setItem('beatforge:jobs', JSON.stringify(jobs.value))
  }

  function addJob(input: StartJobInput): GenerationJob {
    const job: GenerationJob = {
      ...input,
      id: `BF-DEMO-${Date.now()}`,
      status: 'queued',
      progress: 0,
      stage: 'Waiting for browser mock',
      createdAt: 'Just now',
    }
    jobs.value.unshift(job)
    persistBrowserJobs()
    return job
  }

  async function createJob(input: StartJobInput) {
    if (desktop) {
      const job = await createDesktopJob(input)
      upsert(job)
      return job
    }
    const job = addJob(input)
    startBrowserJob(job.id)
    return job
  }

  function startBrowserJob(id: string) {
    const job = jobs.value.find((item) => item.id === id)
    if (!job) return
    job.status = 'running'
    job.stage = 'Browser mock progress'
    job.progress = 0
    mockTasks.get(id)?.cancel()
    mockTasks.set(
      id,
      simulateProgress(
        (progress) => {
          job.progress = progress
          job.stage = progress < 75 ? 'Browser mock generating' : 'Browser mock packaging'
          persistBrowserJobs()
        },
        () => {
          job.status = 'completed'
          job.stage = 'Browser prototype result'
          job.progress = 100
          mockTasks.delete(id)
          persistBrowserJobs()
        },
        300,
      ),
    )
  }

  async function cancelJob(id: string) {
    if (desktop) {
      await cancelDesktopJob(id)
      return
    }
    mockTasks.get(id)?.cancel()
    mockTasks.delete(id)
    const job = jobs.value.find((item) => item.id === id)
    if (job) {
      job.status = 'cancelled'
      job.stage = 'Cancelled'
      persistBrowserJobs()
    }
  }

  async function retryJob(id: string) {
    if (desktop) {
      await retryDesktopJob(id)
      const job = jobs.value.find((item) => item.id === id)
      if (job) {
        job.status = 'queued'
        job.progress = 0
        job.stage = 'Waiting for worker'
        job.errorMessage = undefined
        job.validationReport = undefined
      }
      return
    }
    startBrowserJob(id)
  }

  async function removeJob(id: string) {
    if (desktop) await deleteDesktopJob(id)
    mockTasks.get(id)?.cancel()
    mockTasks.delete(id)
    jobs.value = jobs.value.filter((job) => job.id !== id)
    persistBrowserJobs()
  }

  return {
    jobs,
    activeCount,
    completedJobs,
    completedToday,
    initialized,
    loading,
    error,
    initialize,
    refreshJobs,
    addJob,
    createJob,
    cancelJob,
    retryJob,
    removeJob,
  }
})
