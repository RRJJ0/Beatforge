import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { GenerationJob } from '@/types'
import { simulateProgress } from '@/services/mockBackend'

export const useJobsStore = defineStore('jobs', () => {
  const seedJobs: GenerationJob[] = [
    {
      id: 'BF-0241',
      title: 'Starlight Memory',
      artist: 'Lunaria',
      audioFile: 'starlight-memory.mp3',
      model: 'Mapperatorinator Standard',
      difficulty: 5.2,
      status: 'running',
      progress: 64,
      stage: 'Placing hit objects',
      createdAt: 'Today, 14:22',
      eta: '02:18',
    },
    {
      id: 'BF-0240',
      title: 'Neon Rain',
      artist: 'Asterism',
      audioFile: 'neon-rain.wav',
      model: 'Mapperatorinator Standard',
      difficulty: 4.4,
      status: 'queued',
      progress: 0,
      stage: 'Waiting for GPU',
      createdAt: 'Today, 14:19',
    },
    {
      id: 'BF-0239',
      title: 'Daybreak',
      artist: 'Mizuki',
      audioFile: 'daybreak.flac',
      model: 'Mapperatorinator Standard',
      difficulty: 5.8,
      status: 'completed',
      progress: 100,
      stage: 'Exported',
      createdAt: 'Today, 13:48',
    },
    {
      id: 'BF-0238',
      title: 'Broken Signal',
      artist: 'Unknown',
      audioFile: 'broken-signal.mp3',
      model: 'Mapperatorinator Standard',
      difficulty: 4.0,
      status: 'failed',
      progress: 18,
      stage: 'Audio decoding failed · Try converting the file to WAV.',
      createdAt: 'Today, 12:51',
    },
  ]
  const saved = localStorage.getItem('beatforge:jobs')
  const jobs = ref<GenerationJob[]>(saved ? JSON.parse(saved) : seedJobs)
  const activeCount = computed(
    () => jobs.value.filter((job) => ['running', 'queued'].includes(job.status)).length,
  )
  const completedJobs = computed(() => jobs.value.filter((job) => job.status === 'completed'))
  function addJob(
    input: Pick<GenerationJob, 'title' | 'artist' | 'audioFile' | 'model' | 'difficulty'>,
  ) {
    jobs.value.unshift({
      ...input,
      id: `BF-${String(242 + jobs.value.length).padStart(4, '0')}`,
      status: 'queued',
      progress: 0,
      stage: 'Preparing audio',
      createdAt: 'Just now',
    })
    persist()
  }
  function removeJob(id: string) {
    jobs.value = jobs.value.filter((job) => job.id !== id)
    persist()
  }
  function persist() {
    localStorage.setItem('beatforge:jobs', JSON.stringify(jobs.value))
  }
  function startJob(id: string) {
    const job = jobs.value.find((item) => item.id === id)
    if (!job) return
    job.status = 'running'
    job.stage = 'Analyzing audio'
    job.progress = 0
    persist()
    simulateProgress(
      (progress) => {
        job.progress = progress
        job.stage =
          progress < 30
            ? 'Analyzing audio'
            : progress < 75
              ? 'Placing hit objects'
              : 'Validating beatmap'
        persist()
      },
      () => {
        job.status = 'completed'
        job.stage = 'Exported'
        job.progress = 100
        persist()
      },
      300,
    )
  }
  return { jobs, activeCount, completedJobs, addJob, removeJob, startJob }
})
