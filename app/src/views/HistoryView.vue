<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  NAlert,
  NButton,
  NCard,
  NEmpty,
  NIcon,
  NInput,
  NPagination,
  NSelect,
  NSpin,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  FolderOpenOutline,
  EyeOutline,
  OpenOutline,
  RefreshOutline,
  SearchOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import { useJobsStore } from '@/stores/jobs'
import JobPreviewModal from '@/components/JobPreviewModal.vue'
import { useI18n } from '@/i18n'
import type { GenerationJob, JobStatus } from '@/types'
import {
  importDesktopJobOutput,
  isDesktopRuntime,
  revealDesktopJobOutput,
} from '@/services/desktop'
import {
  filterAndSortJobs,
  formatJobDate,
  type HistoryDateRange,
  type HistorySort,
} from '@/utils/jobHistory'

const PAGE_SIZE = 25
const jobsStore = useJobsStore()
const message = useMessage()
const search = ref('')
const status = ref<'all' | JobStatus>('all')
const model = ref('all')
const dateRange = ref<HistoryDateRange>('all')
const sort = ref<HistorySort>('newest')
const page = ref(1)
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

const statusOptions = computed<Array<{ label: string; value: 'all' | JobStatus }>>(() => [
  { label: t('history.allStatuses'), value: 'all' },
  ...(['completed', 'validation_failed', 'failed', 'cancelled', 'interrupted', 'running', 'validating', 'queued'] as JobStatus[]).map((value) => ({
    label: t(`status.${value}` as 'status.completed'),
    value,
  })),
])

const modelOptions = computed(() => [
  { label: t('history.allModels'), value: 'all' },
  ...[...new Set(jobsStore.jobs.map((job) => job.model))]
    .filter(Boolean)
    .sort((left, right) => left.localeCompare(right))
    .map((value) => ({ label: value, value })),
])

const filtered = computed(() =>
  filterAndSortJobs(jobsStore.jobs, {
    search: search.value,
    status: status.value,
    model: model.value,
    dateRange: dateRange.value,
    sort: sort.value,
  }),
)
const pageCount = computed(() => Math.max(1, Math.ceil(filtered.value.length / PAGE_SIZE)))
const visibleJobs = computed(() => {
  const start = (page.value - 1) * PAGE_SIZE
  return filtered.value.slice(start, start + PAGE_SIZE)
})
const completedCount = computed(
  () =>
    jobsStore.jobs.filter(
      (job) => job.status === 'completed' && job.outputAvailable !== false,
    ).length,
)
const issueCount = computed(
  () =>
    jobsStore.jobs.filter((job) =>
      ['validation_failed', 'failed', 'cancelled', 'interrupted'].includes(job.status),
    ).length,
)
const averageDifficulty = computed(() => {
  if (!jobsStore.jobs.length) return '—'
  const total = jobsStore.jobs.reduce((sum, job) => sum + job.difficulty, 0)
  return `${(total / jobsStore.jobs.length).toFixed(1)}★`
})
const hasFilters = computed(
  () =>
    Boolean(search.value) ||
    status.value !== 'all' ||
    model.value !== 'all' ||
    dateRange.value !== 'all',
)

watch([search, status, model, dateRange, sort], () => {
  page.value = 1
})
watch(pageCount, (count) => {
  if (page.value > count) page.value = count
})

onMounted(() => jobsStore.initialize())

function clearFilters() {
  search.value = ''
  status.value = 'all'
  model.value = 'all'
  dateRange.value = 'all'
  sort.value = 'newest'
}

async function runAction(action: () => Promise<unknown>, success?: string) {
  try {
    await action()
    if (success) message.success(success)
  } catch (reason) {
    message.error(String(reason))
  }
}

async function regenerate(job: GenerationJob) {
  const confirmed = window.confirm(
    `Create a new job using the saved settings from ${job.id}? ` +
      'The original job will remain unchanged. Its audio and reference files must still exist.',
  )
  if (!confirmed) return
  try {
    const created = await jobsStore.regenerateJob(job.id)
    message.success(`Created ${created.id}`)
  } catch (reason) {
    message.error(String(reason))
  }
}

async function remove(job: GenerationJob) {
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
      <div class="stat-label">{{ t('history.stored') }}</div>
      <div class="stat-value">{{ jobsStore.jobs.length }}</div>
      <div class="stat-note">{{ t('history.storedHelp') }}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">{{ t('history.completed') }}</div>
      <div class="stat-value">{{ completedCount }}</div>
      <div class="stat-note">{{ t('history.deliverables') }}</div>
    </div>
    <div class="stat-card">
      <div class="stat-label">{{ t('history.average') }}</div>
      <div class="stat-value">{{ averageDifficulty }}</div>
      <div class="stat-note">{{ t('history.stopped', { count: issueCount }) }}</div>
    </div>
  </div>

  <NCard class="panel-card" :bordered="false">
    <div class="history-heading">
      <div>
        <h2 class="section-title">{{ t('history.title') }}</h2>
        <p class="section-copy">{{ t('history.help') }}</p>
      </div>
      <NButton
        secondary
        :loading="jobsStore.loading"
        :disabled="!isDesktopRuntime()"
        @click="runAction(jobsStore.refreshJobs, 'History refreshed')"
      >
        <template #icon><NIcon :component="RefreshOutline" /></template>
        Refresh
      </NButton>
    </div>

    <div class="history-toolbar">
      <NInput
        v-model:value="search"
        :placeholder="t('history.search')"
        clearable
      >
        <template #prefix><NIcon :component="SearchOutline" /></template>
      </NInput>
      <NSelect v-model:value="status" :options="statusOptions" />
      <NSelect v-model:value="model" :options="modelOptions" />
      <NSelect
        v-model:value="dateRange"
        :options="[
          { label: t('history.allDates'), value: 'all' },
          { label: t('history.today'), value: 'today' },
          { label: t('history.sevenDays'), value: 'seven_days' },
          { label: t('history.thirtyDays'), value: 'thirty_days' },
        ]"
      />
      <NSelect
        v-model:value="sort"
        :options="[
          { label: t('history.newest'), value: 'newest' },
          { label: t('history.oldest'), value: 'oldest' },
          { label: t('history.titleSort'), value: 'title' },
        ]"
      />
    </div>

    <NAlert v-if="jobsStore.error" type="error" class="history-alert">
      Could not load history: {{ jobsStore.error }}
    </NAlert>

    <NSpin :show="jobsStore.loading">
      <div v-if="visibleJobs.length" class="history-table-wrap">
        <div class="history-table">
          <div class="table-row table-head">
            <span>{{ t('history.beatmap') }}</span><span>{{ t('history.status') }}</span><span>{{ t('history.difficulty') }}</span><span>{{ t('history.model') }}</span
            ><span>{{ t('history.created') }}</span><span></span>
          </div>
          <div v-for="job in visibleJobs" :key="job.id" class="table-row">
            <div class="map-cell">
              <div class="map-cover">{{ job.title.slice(0, 1).toUpperCase() }}</div>
              <div class="map-copy">
                <strong>{{ job.title }}</strong>
                <small
                  >{{ job.artist }} · <span class="mono">{{ job.id }}</span></small
                >
                <small class="audio-path">{{ job.audioFile }}</small>
                <small
                  v-if="job.status === 'completed' && job.outputPath && job.outputAvailable === false"
                  class="missing-output"
                >
                  {{ t('jobs.outputMissing') }}
                </small>
              </div>
            </div>
            <div>
              <NTag size="small" round :type="statusType[job.status]">
                {{ t(`status.${job.status}` as 'status.completed') }}
              </NTag>
            </div>
            <div>
              <NTag size="small" round>{{ job.difficulty.toFixed(1) }} ★</NTag>
            </div>
            <span class="secondary model-name">{{ job.model }}</span>
            <span class="secondary">{{ formatJobDate(job.createdAt) }}</span>
            <div class="row-actions">
              <NButton
                v-if="job.status === 'completed' && job.outputPath && job.outputAvailable !== false && isDesktopRuntime()"
                secondary
                circle
                size="small"
                :title="t('jobs.preview')"
                :aria-label="t('jobs.preview')"
                @click="openPreview(job.id)"
              >
                <template #icon><NIcon :component="EyeOutline" /></template>
              </NButton>
              <NButton
                v-if="job.status === 'completed' && job.outputPath && job.outputAvailable !== false && isDesktopRuntime()"
                quaternary
                circle
                size="small"
                :title="t('jobs.show')"
                :aria-label="t('jobs.show')"
                @click="runAction(() => revealDesktopJobOutput(job.id))"
              >
                <template #icon><NIcon :component="FolderOpenOutline" /></template>
              </NButton>
              <NButton
                v-if="job.status === 'completed' && job.outputPath && job.outputAvailable !== false && isDesktopRuntime()"
                secondary
                circle
                size="small"
                :title="t('jobs.import')"
                :aria-label="t('jobs.import')"
                @click="runAction(() => importDesktopJobOutput(job.id), 'Import opened')"
              >
                <template #icon><NIcon :component="OpenOutline" /></template>
              </NButton>
              <NButton
                v-if="
                  !['running', 'queued', 'validating'].includes(job.status) && isDesktopRuntime()
                "
                quaternary
                circle
                size="small"
                :title="t('history.regenerate')"
                :aria-label="t('history.regenerate')"
                @click="regenerate(job)"
              >
                <template #icon><NIcon :component="RefreshOutline" /></template>
              </NButton>
              <NButton
                v-if="!['running', 'queued', 'validating'].includes(job.status)"
                quaternary
                circle
                size="small"
                type="error"
                :title="t('history.delete')"
                :aria-label="t('history.delete')"
                @click="remove(job)"
              >
                <template #icon><NIcon :component="TrashOutline" /></template>
              </NButton>
            </div>
          </div>
        </div>
      </div>
      <NEmpty
        v-else
        class="empty-results"
        :description="hasFilters ? t('history.noMatch') : t('history.empty')"
      >
        <template v-if="hasFilters" #extra>
          <NButton size="small" @click="clearFilters">{{ t('history.clear') }}</NButton>
        </template>
      </NEmpty>
    </NSpin>

    <div v-if="filtered.length > PAGE_SIZE" class="history-footer">
      <span>Showing {{ visibleJobs.length }} of {{ filtered.length }} matching jobs</span>
      <NPagination v-model:page="page" :page-count="pageCount" />
    </div>
  </NCard>
  <JobPreviewModal v-model:show="previewVisible" :job-id="previewJobId" />
</template>

<style scoped>
.history-heading {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 20px;
  margin-bottom: 18px;
}
.history-toolbar {
  display: grid;
  grid-template-columns: minmax(240px, 1.6fr) repeat(4, minmax(130px, 0.7fr));
  gap: 10px;
  margin-bottom: 18px;
}
.history-alert {
  margin-bottom: 16px;
}
.history-table-wrap {
  overflow-x: auto;
}
.history-table {
  width: 100%;
  min-width: 940px;
}
.table-row {
  display: grid;
  grid-template-columns: minmax(260px, 1.5fr) 130px 100px minmax(150px, 0.8fr) 165px 150px;
  gap: 14px;
  align-items: center;
  min-height: 76px;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
  font-size: 11px;
}
.table-head {
  min-height: 36px;
  border-top: 0;
  color: #858da0;
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.07em;
}
.map-cell {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 11px;
}
.map-cover {
  width: 40px;
  height: 40px;
  flex: 0 0 40px;
  display: grid;
  place-items: center;
  border-radius: 9px;
  color: #fff;
  font-weight: 750;
  background: linear-gradient(145deg, #ff66aa, #765ee9);
}
.map-copy {
  min-width: 0;
}
.map-copy strong,
.map-copy small {
  display: block;
}
.map-copy strong {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.map-copy small,
.secondary {
  margin-top: 3px;
  color: #858da0;
  font-size: 10px;
}
.audio-path,
.model-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.map-copy .missing-output {
  color: #d03050;
  white-space: normal;
}
.row-actions {
  display: flex;
  justify-content: flex-end;
  gap: 3px;
}
.empty-results {
  padding: 60px 0;
}
.history-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  margin-top: 18px;
  color: #858da0;
  font-size: 10px;
}
@media (max-width: 1180px) {
  .history-toolbar {
    grid-template-columns: 1fr 1fr 1fr;
  }
  .history-toolbar > :first-child {
    grid-column: 1 / -1;
  }
}
</style>
