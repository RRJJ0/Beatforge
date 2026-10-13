<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { NAlert, NModal, NSelect, NSpin, NTag } from 'naive-ui'
import type { DifficultyPreview, JobPreview } from '@/types'
import { getDesktopJobPreview } from '@/services/desktop'
import { downsamplePeaks, formatPreviewTime } from '@/utils/audioPreview'

const props = defineProps<{ show: boolean; jobId: string | null }>()
const emit = defineEmits<{ 'update:show': [value: boolean] }>()
const audio = ref<HTMLAudioElement | null>(null)
const preview = ref<(JobPreview & { audioUrl: string | null }) | null>(null)
const selectedId = ref<string | null>(null)
const loading = ref(false)
const error = ref('')
const audioError = ref('')
const waveform = ref<number[]>([])
const currentSeconds = ref(0)
const durationSeconds = ref(0)
let loadSequence = 0

const selected = computed<DifficultyPreview | null>(() => {
  return preview.value?.difficulties.find((item) => item.id === selectedId.value) ?? null
})
const difficultyOptions = computed(() =>
  (preview.value?.difficulties ?? []).map((item) => ({
    label: `${item.name} · ${item.objectCount} objects`,
    value: item.id,
  })),
)
const timelineMs = computed(() =>
  Math.max(
    durationSeconds.value * 1000,
    selected.value?.lastObjectMs ?? 0,
    (selected.value?.density.length ?? 0) * (selected.value?.binWidthMs ?? 0),
    1,
  ),
)
const densityMaximum = computed(() => Math.max(...(selected.value?.density ?? [0]), 1))
const densityRects = computed(() => {
  const difficulty = selected.value
  if (!difficulty) return []
  return difficulty.density
    .map((count, index) => ({
      x: (index * difficulty.binWidthMs * 1000) / timelineMs.value,
      width: Math.max((difficulty.binWidthMs * 1000) / timelineMs.value, 1),
      height: (count / densityMaximum.value) * 82,
      count,
    }))
    .filter((item) => item.count > 0)
})
const waveformBars = computed(() =>
  waveform.value.map((peak, index) => ({
    x: ((index + 0.5) * 1000) / waveform.value.length,
    y: 50 - peak * 42,
    height: Math.max(peak * 84, 1),
  })),
)
const playheadX = computed(() => (currentSeconds.value * 1000 * 1000) / timelineMs.value)

async function decodeWaveform(url: string, maxPoints: number, sequence: number) {
  try {
    const response = await fetch(url)
    if (!response.ok) throw new Error(`Audio request failed (${response.status})`)
    const bytes = await response.arrayBuffer()
    const context = new AudioContext()
    try {
      const buffer = await context.decodeAudioData(bytes)
      if (sequence === loadSequence) {
        waveform.value = downsamplePeaks(buffer.getChannelData(0), maxPoints)
      }
    } finally {
      await context.close()
    }
  } catch {
    if (sequence === loadSequence) audioError.value = 'Waveform is unavailable for this audio format.'
  }
}

async function load() {
  const jobId = props.jobId
  if (!props.show || !jobId) return
  const sequence = ++loadSequence
  loading.value = true
  error.value = ''
  audioError.value = ''
  waveform.value = []
  currentSeconds.value = 0
  durationSeconds.value = 0
  preview.value = null
  try {
    const result = await getDesktopJobPreview(jobId)
    if (sequence !== loadSequence) return
    preview.value = result
    selectedId.value = result.difficulties[0]?.id ?? null
    await nextTick()
    if (result.audioUrl && result.waveformAllowed) {
      void decodeWaveform(result.audioUrl, result.maxWaveformPoints, sequence)
    }
  } catch (reason) {
    if (sequence === loadSequence) error.value = String(reason)
  } finally {
    if (sequence === loadSequence) loading.value = false
  }
}

function close() {
  audio.value?.pause()
  emit('update:show', false)
}

function updateAudioTime() {
  currentSeconds.value = audio.value?.currentTime ?? 0
}

function updateAudioDuration() {
  durationSeconds.value = Number.isFinite(audio.value?.duration) ? (audio.value?.duration ?? 0) : 0
}

function handleAudioError() {
  audioError.value = 'The browser could not play this audio format. Density preview is still available.'
}

function seek(event: MouseEvent) {
  const element = event.currentTarget as HTMLElement
  const ratio = Math.min(Math.max((event.clientX - element.getBoundingClientRect().left) / element.clientWidth, 0), 1)
  const seconds = (timelineMs.value * ratio) / 1000
  currentSeconds.value = seconds
  if (audio.value && Number.isFinite(audio.value.duration)) audio.value.currentTime = Math.min(seconds, audio.value.duration)
}

watch(() => [props.show, props.jobId], load, { immediate: true })
onBeforeUnmount(() => {
  loadSequence += 1
  audio.value?.pause()
})
</script>

<template>
  <NModal
    :show="show"
    preset="card"
    class="preview-modal"
    title="Result preview"
    :bordered="false"
    @update:show="(value) => (value ? undefined : close())"
  >
    <NSpin :show="loading">
      <NAlert v-if="error" type="error">{{ error }}</NAlert>
      <template v-else-if="preview && selected">
        <div class="preview-heading">
          <NSelect v-model:value="selectedId" :options="difficultyOptions" />
          <NTag round size="small">{{ selected.objectCount }} objects</NTag>
        </div>

        <NAlert v-if="preview.audioUnavailableReason" type="warning" class="preview-alert">
          {{ preview.audioUnavailableReason }} Density preview remains available.
        </NAlert>
        <NAlert v-else-if="audioError" type="warning" class="preview-alert">
          {{ audioError }}
        </NAlert>

        <audio
          v-if="preview.audioUrl"
          ref="audio"
          class="preview-audio"
          :src="preview.audioUrl"
          controls
          preload="metadata"
          @loadedmetadata="updateAudioDuration"
          @durationchange="updateAudioDuration"
          @timeupdate="updateAudioTime"
          @error="handleAudioError"
        ></audio>

        <div class="timeline" role="slider" tabindex="0" aria-label="Preview timeline" @click="seek">
          <svg viewBox="0 0 1000 100" preserveAspectRatio="none" aria-hidden="true">
            <rect
              v-for="(bar, index) in densityRects"
              :key="`density-${index}`"
              :x="bar.x"
              :y="100 - bar.height"
              :width="bar.width"
              :height="bar.height"
              class="density-bar"
            />
            <rect
              v-for="(bar, index) in waveformBars"
              :key="`wave-${index}`"
              :x="bar.x"
              :y="bar.y"
              width="1.5"
              :height="bar.height"
              class="waveform-bar"
            />
            <line :x1="playheadX" y1="0" :x2="playheadX" y2="100" class="playhead" />
          </svg>
        </div>
        <div class="timeline-meta">
          <span>{{ formatPreviewTime(currentSeconds) }} / {{ formatPreviewTime(timelineMs / 1000) }}</span>
          <span v-if="selected.firstObjectMs != null">
            First object {{ formatPreviewTime(selected.firstObjectMs / 1000) }}
          </span>
          <span v-else>No hit objects</span>
        </div>
        <p class="preview-note">
          Pink bars show validated hit-object density. This preview is read-only; use osu! Editor for editing.
        </p>
      </template>
    </NSpin>
  </NModal>
</template>

<style scoped>
:global(.preview-modal) {
  width: min(760px, calc(100vw - 32px));
}
.preview-heading {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
}
.preview-alert,
.preview-audio {
  width: 100%;
  margin-top: 16px;
}
.timeline {
  height: 190px;
  margin-top: 18px;
  overflow: hidden;
  cursor: pointer;
  border: 1px solid rgba(255, 102, 170, 0.22);
  border-radius: 12px;
  background: linear-gradient(180deg, rgba(118, 94, 233, 0.12), rgba(255, 102, 170, 0.04));
}
.timeline svg {
  width: 100%;
  height: 100%;
}
.density-bar {
  fill: rgba(255, 102, 170, 0.72);
}
.waveform-bar {
  fill: rgba(205, 214, 244, 0.42);
}
.playhead {
  stroke: #fff;
  stroke-width: 2;
}
.timeline-meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-top: 8px;
  color: #858da0;
  font-size: 11px;
}
.preview-note {
  margin: 18px 0 0;
  color: #858da0;
  font-size: 11px;
}
</style>
