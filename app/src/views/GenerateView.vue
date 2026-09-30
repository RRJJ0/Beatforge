<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NAlert, NButton, NCard, NForm, NFormItem, NIcon, NInput, NInputNumber,
  NSelect, NSlider, NSwitch, NTag, NUpload, NUploadDragger, useMessage,
} from 'naive-ui'
import { CloudUploadOutline, MusicalNotesOutline, PlayOutline, SparklesOutline } from '@vicons/ionicons5'
import { useJobsStore } from '@/stores/jobs'

const router = useRouter()
const message = useMessage()
const jobsStore = useJobsStore()
const audioFile = ref('')
const title = ref('')
const artist = ref('')
const model = ref('standard')
const difficulty = ref(4.8)
const approachRate = ref(9)
const circleSize = ref(4)
const generateHitsounds = ref(true)
const isAdvanced = ref(false)
const attempted = ref(false)

const modelOptions = [
  { label: 'Mapperatorinator Standard · v1.3', value: 'standard' },
  { label: 'Mapperatorinator High SR · v1.1', value: 'high-sr' },
]
const canGenerate = computed(() => Boolean(audioFile.value && title.value.trim() && artist.value.trim()))

function onFileChange(options: { fileList: Array<{ name: string }> }) {
  const file = options.fileList.at(-1)
  audioFile.value = file?.name ?? ''
  if (file && !title.value) title.value = file.name.replace(/\.[^/.]+$/, '').replaceAll('-', ' ')
}

function createJob() {
  attempted.value = true
  if (!canGenerate.value) {
    message.warning('Add an audio file, title, and artist before generating.')
    return
  }
  jobsStore.addJob({
    title: title.value.trim(), artist: artist.value.trim(), audioFile: audioFile.value,
    model: model.value === 'high-sr' ? 'Mapperatorinator High SR' : 'Mapperatorinator Standard',
    difficulty: difficulty.value,
  })
  message.success('Generation job added to the queue.')
  router.push('/jobs')
}
</script>

<template>
  <div class="generate-grid">
    <section class="generate-main">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div><h2 class="section-title">Source audio</h2><p class="section-copy">Choose a clean, full-length track for the best result.</p></div>
          <NTag v-if="audioFile" size="small" type="success">Audio ready</NTag>
        </div>
        <NUpload accept="audio/*,.mp3,.wav,.ogg,.flac" :max="1" @change="onFileChange">
          <NUploadDragger class="audio-dropzone">
            <div class="upload-icon"><NIcon :component="audioFile ? MusicalNotesOutline : CloudUploadOutline" /></div>
            <strong>{{ audioFile || 'Drop your audio file here' }}</strong>
            <p>{{ audioFile ? 'Ready for analysis · click to replace' : 'or click to browse · MP3, WAV, OGG, FLAC' }}</p>
          </NUploadDragger>
        </NUpload>
        <NAlert v-if="attempted && !audioFile" class="validation-alert" type="error" :show-icon="false">Select an audio file to continue.</NAlert>
      </NCard>

      <NCard class="panel-card" :bordered="false">
        <div class="card-heading"><div><h2 class="section-title">Beatmap details</h2><p class="section-copy">These fields will be written into the .osu metadata.</p></div></div>
        <NForm label-placement="top">
          <div class="two-column">
            <NFormItem label="Title" required :validation-status="attempted && !title.trim() ? 'error' : undefined" :feedback="attempted && !title.trim() ? 'Enter the song title' : undefined"><NInput v-model:value="title" placeholder="Song title" /></NFormItem>
            <NFormItem label="Artist" required :validation-status="attempted && !artist.trim() ? 'error' : undefined" :feedback="attempted && !artist.trim() ? 'Enter the artist name' : undefined"><NInput v-model:value="artist" placeholder="Artist name" /></NFormItem>
          </div>
          <NFormItem label="Creator"><NInput value="Beatforge AI" disabled /></NFormItem>
        </NForm>
      </NCard>
    </section>

    <aside class="generate-side">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading"><div><h2 class="section-title">Generation setup</h2><p class="section-copy">Tune the target play experience.</p></div></div>
        <NForm label-placement="top">
          <NFormItem label="Model"><NSelect v-model:value="model" :options="modelOptions" /></NFormItem>
          <NFormItem label="Target star rating">
            <div class="slider-field"><NSlider v-model:value="difficulty" :min="1" :max="8" :step="0.1" /><NInputNumber v-model:value="difficulty" :min="1" :max="8" :step="0.1" size="small" /></div>
          </NFormItem>
          <div class="switch-row"><div><strong>Advanced controls</strong><small>Adjust AR, CS, and hitsounds</small></div><NSwitch v-model:value="isAdvanced" /></div>
          <template v-if="isAdvanced">
            <div class="two-column compact">
              <NFormItem label="Approach rate"><NInputNumber v-model:value="approachRate" :min="0" :max="10" /></NFormItem>
              <NFormItem label="Circle size"><NInputNumber v-model:value="circleSize" :min="0" :max="10" /></NFormItem>
            </div>
            <div class="switch-row"><div><strong>Generate hitsounds</strong><small>Add model-predicted samples</small></div><NSwitch v-model:value="generateHitsounds" /></div>
          </template>
        </NForm>
        <NAlert class="local-note" type="info" :show-icon="false">Generation runs locally. Your audio never leaves this computer.</NAlert>
        <NButton type="primary" size="large" block @click="createJob">
          <template #icon><NIcon :component="SparklesOutline" /></template>Generate beatmap
        </NButton>
        <div class="estimate"><NIcon :component="PlayOutline" /><span>Estimated time: 3–6 minutes</span></div>
      </NCard>
    </aside>
  </div>
</template>

<style scoped>
.generate-grid { display: grid; grid-template-columns: minmax(0, 1.55fr) minmax(320px, .8fr); gap: 20px; align-items: start; }
.generate-main { display: grid; gap: 20px; }
.audio-dropzone { min-height: 190px; display: flex; flex-direction: column; justify-content: center; border-radius: 12px; }
.upload-icon { width: 50px; height: 50px; margin: 0 auto 13px; display: grid; place-items: center; border-radius: 14px; color: #ff66aa; background: rgba(255,102,170,.13); font-size: 25px; }
.audio-dropzone strong { font-size: 14px; }.audio-dropzone p { margin: 7px 0 0; color: #858da0; font-size: 11px; }
.two-column { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }.two-column.compact { gap: 10px; }
.slider-field { display: grid; grid-template-columns: 1fr 82px; gap: 14px; align-items: center; width: 100%; }
.switch-row { display: flex; justify-content: space-between; align-items: center; padding: 13px 0; border-top: 1px solid rgba(127,127,127,.12); }
.switch-row strong,.switch-row small { display: block; }.switch-row strong { font-size: 12px; }.switch-row small { margin-top: 3px; color: #858da0; font-size: 10px; }
.local-note { margin: 16px 0; font-size: 11px; }.estimate { display: flex; gap: 6px; justify-content: center; align-items: center; margin-top: 11px; color: #858da0; font-size: 10px; }
.validation-alert { margin-top: 10px; font-size: 11px; }
@media (max-width: 1180px) { .generate-grid { grid-template-columns: 1fr 340px; } }
</style>

