<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { NButton, NCard, NEmpty, NIcon, NProgress, NTag, useMessage } from 'naive-ui'
import {
  CloseOutline,
  EyeOutline,
  FolderOpenOutline,
  OpenOutline,
  PlayOutline,
  RefreshOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import { useJobsStore } from '@/stores/jobs'
import { useDesktopStore } from '@/stores/desktop'
import JobPreviewModal from '@/components/JobPreviewModal.vue'
import { useI18n } from '@/i18n'
import type { BeatmapStatistics, GenerationJob } from '@/types'
import {
  importDesktopJobOutput,
  importFailedDesktopJobOutput,
  revealDesktopJobOutput,
} from '@/services/desktop'

const jobsStore = useJobsStore()
const desktopStore = useDesktopStore()
const message = useMessage()
const route = useRoute()
const previewJobId = ref<string | null>(null)
const previewVisible = ref(false)
const { t } = useI18n()
const statusType = {
  running: 'info',
  validating: 'info',
  queued: 'warning',
  completed: 'success',
  validation_failed: 'error',
  failed: 'error',
  cancelled: 'default',
  interrupted: 'warning',
} as const

watch(
  [() => route.query.job, () => jobsStore.jobs.length],
  async ([jobId]) => {
    if (typeof jobId !== 'string' || !jobId) return
    await nextTick()
    document.getElementById(`job-${jobId}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' })
  },
  { immediate: true },
)

async function runAction(action: () => Promise<void>, success?: string) {
  try {
    await action()
    if (success) message.success(success)
  } catch (reason) {
    message.error(String(reason))
  }
}

function objectCount(statistics: BeatmapStatistics) {
  return statistics.circleCount + statistics.sliderCount + statistics.spinnerCount
}

function bpmSummary(statistics: BeatmapStatistics) {
  if (statistics.bpmMin == null || statistics.bpmMax == null) return 'BPM —'
  const minimum = statistics.bpmMin.toFixed(1).replace(/\.0$/, '')
  const maximum = statistics.bpmMax.toFixed(1).replace(/\.0$/, '')
  return minimum === maximum ? `${minimum} BPM` : `${minimum}–${maximum} BPM`
}

function durationSummary(milliseconds?: number | null) {
  if (milliseconds == null) return 'Duration —'
  const totalSeconds = Math.round(milliseconds / 1000)
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = String(totalSeconds % 60).padStart(2, '0')
  return `${minutes}:${seconds}`
}

async function importFailedOutput(jobId: string, errorCount: number) {
  const confirmed = window.confirm(
    `This beatmap failed validation with ${errorCount} error${errorCount === 1 ? '' : 's'}. ` +
      'It may be unplayable, incomplete, or rejected by osu!. Import the raw generated file anyway?',
  )
  if (!confirmed) return
  await runAction(
    () => importFailedDesktopJobOutput(jobId).then(() => undefined),
    'Raw output sent to osu!',
  )
}

async function removeJob(job: GenerationJob) {
  const confirmed = window.confirm(
    `Permanently delete ${job.id} and its managed raw, validated, report, log, and temporary files? ` +
      'The original audio and reference files outside Beatforge will not be deleted.',
  )
  if (!confirmed) return
  await runAction(() => jobsStore.removeJob(job.id), 'Job and managed files deleted')
}

function openPreview(jobId: string) {
  previewJobId.value = jobId
  previewVisible.value = true
}
</script>

<template>
  <div class="stat-grid">
    <div class="stat-card">
      <div class="stat-label">{{ t('jobs.active') }}</div>
      <div class="stat-value">{{ jobsStore.activeCount }}</div>
      <div class="stat-note">{{ t('jobs.queueNote') }}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">{{ t('jobs.gpu') }}</div>
      <div class="stat-value">{{ desktopStore.system?.gpu.available ? 'Ready' : '—' }}</div>
      <div class="stat-note">{{ desktopStore.system?.gpu.name ?? t('common.checking') }}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">{{ t('jobs.completedToday') }}</div>
      <div class="stat-value">{{ jobsStore.completedToday }}</div>
      <div class="stat-note">{{ t('jobs.persisted') }}</div>
    </div>
  </div>

  <NCard class="panel-card" :bordered="false">
    <div class="card-heading">
      <div>
        <h2 class="section-title">{{ t('jobs.queue') }}</h2>
        <p class="section-copy">{{ t('jobs.queueHelp') }}</p>
      </div>
      <NTag size="small" round type="info">Protocol v1</NTag>
    </div>
    <div v-if="jobsStore.jobs.length" class="job-list">
      <article
        v-for="job in jobsStore.jobs"
        :id="`job-${job.id}`"
        :key="job.id"
        class="job-row"
        :class="{ 'is-notification-target': route.query.job === job.id }"
      >
        <div class="job-art"><NIcon :component="PlayOutline" /></div>
        <div class="job-content">
          <div class="job-title-row">
            <div>
              <strong>{{ job.title }}</strong
              ><span>{{ job.artist }} · {{ job.audioFile }}</span>
            </div>
            <NTag size="small" round :type="statusType[job.status]">{{ t(`status.${job.status}` as 'status.completed') }}</NTag>
          </div>
          <div class="job-meta">
            <span class="mono">{{ job.id }}</span
            ><span>{{ job.model }}</span
            ><span>★ {{ job.difficulty.toFixed(1) }}</span
            ><span>{{ job.createdAt }}</span>
          </div>
          <template v-if="['running', 'validating'].includes(job.status)">
            <div class="progress-label">
              <span>{{ job.stage }}</span
              ><span>{{ job.progress }}% · ETA {{ job.eta }}</span>
            </div>
            <NProgress
              type="line"
              :percentage="job.progress"
              :height="7"
              :show-indicator="false"
              color="#ff66aa"
            />
          </template>
          <div v-else class="job-stage">{{ job.stage }}</div>
          <div v-if="job.errorMessage" class="job-error">{{ job.errorMessage }}</div>
          <div v-if="job.outputPath" class="job-output mono">{{ job.outputPath }}</div>
          <div v-if="job.provenance" class="job-provenance">
            <span class="mono">Run {{ job.provenance.reproduction_key.slice(0, 12) }}</span>
            <span>{{ job.provenance.runtime.gpu_name ?? job.provenance.runtime.device }}</span>
            <span>{{ job.provenance.runtime.precision.toUpperCase() }}</span>
            <span>{{ job.provenance.timing.elapsed_seconds.toFixed(1) }}s</span>
            <span v-if="job.provenance.runtime.peak_gpu_memory_mb">
              Peak {{ job.provenance.runtime.peak_gpu_memory_mb.toFixed(0) }} MB
            </span>
          </div>
          <div v-if="job.validationReport" class="job-validation">
            <span>{{ job.validationReport.status.replaceAll('_', ' ') }}</span>
            <span>
              {{ job.validationReport.issues.filter((issue) => issue.severity === 'error').length }}
              errors
            </span>
            <span>
              {{
                job.validationReport.issues.filter((issue) => issue.severity === 'warning').length
              }}
              warnings
            </span>
          </div>
          <div v-if="job.validationReport?.statistics" class="job-statistics">
            <span>{{ job.validationReport.statistics.beatmapCount }} difficulty</span>
            <span>{{ objectCount(job.validationReport.statistics) }} objects</span>
            <span>{{ bpmSummary(job.validationReport.statistics) }}</span>
            <span>{{ durationSummary(job.validationReport.statistics.durationMs) }}</span>
            <span v-if="job.validationReport.statistics.maxCombo">
              {{ job.validationReport.statistics.maxCombo }}× combo
            </span>
            <span v-if="job.validationReport.statistics.starRating">
              ★ {{ job.validationReport.statistics.starRating.toFixed(2) }}
            </span>
          </div>
        </div>
        <div class="job-actions">
          <NButton
            v-if="job.status === 'completed' && job.outputPath"
            secondary
            circle
            size="small"
            :title="t('jobs.preview')"
            :aria-label="t('jobs.preview')"
            @click="openPreview(job.id)"
            ><template #icon><NIcon :component="EyeOutline" /></template
          ></NButton>
          <NButton
            v-if="job.status === 'completed' && job.outputPath"
            quaternary
            circle
            size="small"
            :title="t('jobs.show')"
            :aria-label="t('jobs.show')"
            @click="runAction(() => revealDesktopJobOutput(job.id))"
            ><template #icon><NIcon :component="FolderOpenOutline" /></template
          ></NButton>
          <NButton
            v-if="job.status === 'completed' && job.outputPath"
            secondary
            circle
            size="small"
            :title="t('jobs.import')"
            :aria-label="t('jobs.import')"
            @click="
              runAction(() => importDesktopJobOutput(job.id).then(() => undefined), 'Import opened')
            "
            ><template #icon><NIcon :component="OpenOutline" /></template
          ></NButton>
          <NButton
            v-if="job.status === 'validation_failed' && job.validationReport"
            secondary
            circle
            size="small"
            type="warning"
            :title="t('jobs.importFailed')"
            :aria-label="t('jobs.importFailed')"
            @click="
              importFailedOutput(
                job.id,
                job.validationReport.issues.filter((issue) => issue.severity === 'error').length,
              )
            "
            ><template #icon><NIcon :component="OpenOutline" /></template
          ></NButton>
          <NButton
            v-if="['running', 'queued'].includes(job.status)"
            quaternary
            circle
            size="small"
            :aria-label="t('jobs.cancel')"
            @click="runAction(() => jobsStore.cancelJob(job.id))"
            ><template #icon><NIcon :component="CloseOutline" /></template
          ></NButton>
          <NButton
            v-if="['failed', 'validation_failed', 'cancelled', 'interrupted'].includes(job.status)"
            secondary
            circle
            size="small"
            :aria-label="t('jobs.retry')"
            @click="runAction(() => jobsStore.retryJob(job.id), 'Job queued again')"
            ><template #icon><NIcon :component="RefreshOutline" /></template
          ></NButton>
          <NButton
            v-if="!['running', 'queued'].includes(job.status)"
            quaternary
            circle
            size="small"
            :aria-label="t('jobs.delete')"
            @click="removeJob(job)"
            ><template #icon><NIcon :component="TrashOutline" /></template
          ></NButton>
        </div>
      </article>
    </div>
    <NEmpty v-else :description="t('jobs.empty')" />
  </NCard>
  <JobPreviewModal v-model:show="previewVisible" :job-id="previewJobId" />
</template>

<style scoped>
.job-list {
  display: grid;
  gap: 1px;
}
.job-row {
  display: grid;
  grid-template-columns: 48px 1fr 34px;
  gap: 15px;
  align-items: start;
  padding: 18px 0;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.job-row:first-child {
  border-top: 0;
  padding-top: 2px;
}
.job-row.is-notification-target {
  margin-inline: -10px;
  padding-inline: 10px;
  border-radius: 10px;
  background: rgba(255, 102, 170, 0.08);
  box-shadow: inset 3px 0 #ff66aa;
}
.job-art {
  width: 48px;
  height: 48px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  color: #ff66aa;
  background: linear-gradient(145deg, rgba(255, 102, 170, 0.18), rgba(151, 103, 255, 0.12));
}
.job-title-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}
.job-title-row strong,
.job-title-row span {
  display: block;
}
.job-title-row strong {
  font-size: 13px;
}
.job-title-row span {
  margin-top: 2px;
  color: #858da0;
  font-size: 10px;
}
.job-meta {
  display: flex;
  gap: 14px;
  margin-top: 9px;
  color: #858da0;
  font-size: 10px;
}
.progress-label {
  display: flex;
  justify-content: space-between;
  margin: 14px 0 7px;
  font-size: 10px;
}
.progress-label span:last-child {
  color: #858da0;
}
.job-stage {
  margin-top: 12px;
  color: #858da0;
  font-size: 10px;
}
.job-error {
  margin-top: 7px;
  color: #e15b64;
  font-size: 10px;
}
.job-output {
  margin-top: 7px;
  overflow: hidden;
  color: #858da0;
  font-size: 9px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.job-provenance {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 12px;
  margin-top: 9px;
  color: #858da0;
  font-size: 9px;
}
.job-provenance span:first-child {
  color: #ff66aa;
}
.job-validation {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 12px;
  margin-top: 7px;
  color: #858da0;
  font-size: 9px;
  text-transform: capitalize;
}
.job-validation span:first-child {
  color: #66cdaa;
}
.job-statistics {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 12px;
  margin-top: 7px;
  color: #b4bac8;
  font-size: 9px;
}
.job-actions {
  display: grid;
  gap: 4px;
}
</style>
