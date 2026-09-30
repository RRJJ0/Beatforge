<script setup lang="ts">
import { NButton, NCard, NEmpty, NIcon, NProgress, NTag, useMessage } from 'naive-ui'
import { CloseOutline, PauseOutline, PlayOutline, TrashOutline } from '@vicons/ionicons5'
import { useJobsStore } from '@/stores/jobs'

const jobsStore = useJobsStore()
const message = useMessage()
const statusType = { running: 'info', queued: 'warning', completed: 'success', failed: 'error' } as const
</script>

<template>
  <div class="stat-grid">
    <div class="stat-card"><div class="stat-label">Active jobs</div><div class="stat-value">{{ jobsStore.activeCount }}</div><div class="stat-note">1 generating · 1 queued</div></div>
    <div class="stat-card"><div class="stat-label">GPU utilization</div><div class="stat-value">78%</div><div class="stat-note">6.8 / 12 GB VRAM</div></div>
    <div class="stat-card"><div class="stat-label">Completed today</div><div class="stat-value">4</div><div class="stat-note">18 beatmaps this week</div></div>
  </div>

  <NCard class="panel-card" :bordered="false">
    <div class="card-heading"><div><h2 class="section-title">Generation queue</h2><p class="section-copy">Jobs run one at a time to keep GPU memory stable.</p></div><NButton secondary size="small" @click="message.info('Queue paused (prototype)')"><template #icon><NIcon :component="PauseOutline" /></template>Pause queue</NButton></div>
    <div v-if="jobsStore.jobs.length" class="job-list">
      <article v-for="job in jobsStore.jobs" :key="job.id" class="job-row">
        <div class="job-art"><NIcon :component="PlayOutline" /></div>
        <div class="job-content">
          <div class="job-title-row"><div><strong>{{ job.title }}</strong><span>{{ job.artist }} · {{ job.audioFile }}</span></div><NTag size="small" round :type="statusType[job.status]">{{ job.status }}</NTag></div>
          <div class="job-meta"><span class="mono">{{ job.id }}</span><span>{{ job.model }}</span><span>★ {{ job.difficulty.toFixed(1) }}</span><span>{{ job.createdAt }}</span></div>
          <template v-if="job.status === 'running'">
            <div class="progress-label"><span>{{ job.stage }}</span><span>{{ job.progress }}% · ETA {{ job.eta }}</span></div>
            <NProgress type="line" :percentage="job.progress" :height="7" :show-indicator="false" color="#ff66aa" />
          </template>
          <div v-else class="job-stage">{{ job.stage }}</div>
        </div>
        <NButton v-if="job.status === 'queued'" secondary circle size="small" aria-label="Start mock job" @click="jobsStore.startJob(job.id)"><template #icon><NIcon :component="PlayOutline" /></template></NButton>
        <NButton v-else quaternary circle size="small" :aria-label="job.status === 'running' ? 'Cancel job' : 'Remove job'" @click="jobsStore.removeJob(job.id)">
          <template #icon><NIcon :component="job.status === 'running' ? CloseOutline : TrashOutline" /></template>
        </NButton>
      </article>
    </div>
    <NEmpty v-else description="No jobs in the queue" />
  </NCard>
</template>

<style scoped>
.job-list { display: grid; gap: 1px; }.job-row { display: grid; grid-template-columns: 48px 1fr 34px; gap: 15px; align-items: start; padding: 18px 0; border-top: 1px solid rgba(127,127,127,.12); }.job-row:first-child { border-top: 0; padding-top: 2px; }
.job-art { width: 48px; height: 48px; display: grid; place-items: center; border-radius: 11px; color: #ff66aa; background: linear-gradient(145deg, rgba(255,102,170,.18), rgba(151,103,255,.12)); }
.job-title-row { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; }.job-title-row strong,.job-title-row span { display: block; }.job-title-row strong { font-size: 13px; }.job-title-row span { margin-top: 2px; color: #858da0; font-size: 10px; }
.job-meta { display: flex; gap: 14px; margin-top: 9px; color: #858da0; font-size: 10px; }.progress-label { display: flex; justify-content: space-between; margin: 14px 0 7px; font-size: 10px; }.progress-label span:last-child { color: #858da0; }.job-stage { margin-top: 12px; color: #858da0; font-size: 10px; }
</style>

