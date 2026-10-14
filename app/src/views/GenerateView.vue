<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NCard,
  NDynamicTags,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  NSelect,
  NSlider,
  NSwitch,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  CloudUploadOutline,
  DocumentAttachOutline,
  MusicalNotesOutline,
  PlayOutline,
  SparklesOutline,
} from '@vicons/ionicons5'
import { useJobsStore } from '@/stores/jobs'
import { isDesktopRuntime, selectAudioFile, selectBeatmapFile } from '@/services/desktop'
import { useI18n } from '@/i18n'

const router = useRouter()
const message = useMessage()
const jobsStore = useJobsStore()
const { t } = useI18n()
const audioFile = ref('')
const title = ref('')
const artist = ref('')
const model = ref('mapperatorinator-v32-standard')
const isAdvanced = ref(false)
const attempted = ref(false)
const creating = ref(false)
const scenario = ref('success')
const referenceBeatmap = ref('')

interface EditableDifficulty {
  id: string
  name: string
  difficulty: number
  seed: number
  descriptors: string[]
  negativeDescriptors: string[]
  approachRate: number
  circleSize: number
  hitsounded: boolean
}

let nextDifficultyId = 1
function makeDifficulty(rating = 4.8): EditableDifficulty {
  const id = nextDifficultyId++
  return {
    id: `difficulty-${id}`,
    name: `AI ${rating.toFixed(1)}★`,
    difficulty: rating,
    seed: Math.floor(Math.random() * 65_536),
    descriptors: [],
    negativeDescriptors: [],
    approachRate: 9,
    circleSize: 4,
    hitsounded: true,
  }
}
const difficulties = ref<EditableDifficulty[]>([makeDifficulty()])

const modelOptions = [
  { label: 'Mapperatorinator V32 · osu!standard', value: 'mapperatorinator-v32-standard' },
]
const scenarioOptions = [
  { label: 'Production inference', value: 'success' },
  { label: 'Simulate generation failure', value: 'failure' },
  { label: 'Simulate CUDA out of memory', value: 'oom' },
  { label: 'Simulate worker timeout', value: 'hang' },
  { label: 'Simulate unexpected process exit', value: 'crash' },
  { label: 'Simulate malformed protocol message', value: 'malformed' },
]
const canGenerate = computed(() =>
  Boolean(
    audioFile.value &&
      title.value.trim() &&
      artist.value.trim() &&
      difficulties.value.length > 0 &&
      difficulties.value.every((item) => item.name.trim()) &&
      new Set(difficulties.value.map((item) => item.name.trim().toLocaleLowerCase())).size ===
        difficulties.value.length,
  ),
)
const audioDisplayName = computed(() => audioFile.value.split(/[\\/]/).at(-1) ?? '')
const referenceDisplayName = computed(() => referenceBeatmap.value.split(/[\\/]/).at(-1) ?? '')

function addDifficulty() {
  const previous = difficulties.value.at(-1)?.difficulty ?? 3.8
  difficulties.value.push(makeDifficulty(Math.min(8, Number((previous + 0.8).toFixed(1)))))
}

function removeDifficulty(id: string) {
  if (difficulties.value.length > 1) {
    difficulties.value = difficulties.value.filter((item) => item.id !== id)
  }
}

async function chooseAudio() {
  if (!isDesktopRuntime()) {
    message.info(t('generate.pickDesktop'))
    return
  }
  const selected = await selectAudioFile()
  if (!selected) return
  audioFile.value = selected
  const fileName = selected.split(/[\\/]/).at(-1) ?? selected
  if (!title.value) title.value = fileName.replace(/\.[^/.]+$/, '').replaceAll('-', ' ')
}

async function chooseReferenceBeatmap() {
  if (!isDesktopRuntime()) {
    message.info(t('generate.pickDesktop'))
    return
  }
  const selected = await selectBeatmapFile()
  if (selected) referenceBeatmap.value = selected
}

async function createJob() {
  attempted.value = true
  if (!canGenerate.value) {
    message.warning(t('generate.missing'))
    return
  }
  creating.value = true
  try {
    const primary = difficulties.value[0]!
    await jobsStore.createJob({
      title: title.value.trim(),
      artist: artist.value.trim(),
      audioFile: audioFile.value,
      model: model.value,
      difficulty: primary.difficulty,
      difficulties: difficulties.value.map((item) => ({
        ...item,
        name: item.name.trim(),
      })),
      gamemode: 0,
      seed: primary.seed,
      descriptors: primary.descriptors,
      negativeDescriptors: primary.negativeDescriptors,
      referenceBeatmap: referenceBeatmap.value || undefined,
      approachRate: primary.approachRate,
      circleSize: primary.circleSize,
      hitsounded: primary.hitsounded,
      scenario: scenario.value,
    })
    message.success(t('generate.queued'))
    router.push('/jobs')
  } catch (reason) {
    message.error(t('generate.failed', { error: String(reason) }))
  } finally {
    creating.value = false
  }
}
</script>

<template>
  <div class="generate-grid">
    <section class="generate-main">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">{{ t('generate.source') }}</h2>
            <p class="section-copy">{{ t('generate.sourceHelp') }}</p>
          </div>
          <NTag v-if="audioFile" size="small" type="success">{{ t('generate.audioReady') }}</NTag>
        </div>
        <button class="audio-dropzone" type="button" @click="chooseAudio">
          <div class="upload-icon">
            <NIcon :component="audioFile ? MusicalNotesOutline : CloudUploadOutline" />
          </div>
          <strong>{{ audioDisplayName || t('generate.chooseAudio') }}</strong>
          <p>{{ audioFile || t('generate.audioTypes') }}</p>
        </button>
        <NAlert
          v-if="attempted && !audioFile"
          class="validation-alert"
          type="error"
          :show-icon="false"
          >{{ t('generate.audioRequired') }}</NAlert
        >
        <div class="reference-row">
          <div>
            <strong>{{ t('generate.reference') }}</strong>
            <small>{{ referenceDisplayName || t('generate.referenceHelp') }}</small>
          </div>
          <NButton secondary @click="chooseReferenceBeatmap">
            <template #icon><NIcon :component="DocumentAttachOutline" /></template>
            {{ referenceBeatmap ? t('common.change') : t('generate.chooseOsu') }}
          </NButton>
        </div>
      </NCard>

      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">{{ t('generate.details') }}</h2>
            <p class="section-copy">{{ t('generate.detailsHelp') }}</p>
          </div>
        </div>
        <NForm label-placement="top">
          <div class="two-column">
            <NFormItem
              :label="t('generate.title')"
              required
              :validation-status="attempted && !title.trim() ? 'error' : undefined"
              :feedback="attempted && !title.trim() ? t('generate.enterTitle') : undefined"
              ><NInput v-model:value="title" :placeholder="t('generate.songTitle')"
            /></NFormItem>
            <NFormItem
              :label="t('generate.artist')"
              required
              :validation-status="attempted && !artist.trim() ? 'error' : undefined"
              :feedback="attempted && !artist.trim() ? t('generate.enterArtist') : undefined"
              ><NInput v-model:value="artist" :placeholder="t('generate.artistName')"
            /></NFormItem>
          </div>
          <NFormItem :label="t('generate.creator')"><NInput value="Beatforge AI" disabled /></NFormItem>
        </NForm>
      </NCard>
    </section>

    <aside class="generate-side">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">{{ t('generate.setup') }}</h2>
            <p class="section-copy">{{ t('generate.setupHelp') }}</p>
          </div>
        </div>
        <NForm label-placement="top">
          <NFormItem :label="t('generate.model')"
            ><NSelect v-model:value="model" :options="modelOptions"
          /></NFormItem>
          <div class="difficulty-heading">
            <strong>{{ t('generate.difficulties') }}</strong>
            <NButton size="tiny" secondary :disabled="difficulties.length >= 8" @click="addDifficulty">
              {{ t('generate.addDifficulty') }}
            </NButton>
          </div>
          <div class="difficulty-list">
            <div v-for="(item, index) in difficulties" :key="item.id" class="difficulty-card">
              <div class="difficulty-card-heading">
                <span>{{ t('generate.difficultyNumber', { number: index + 1 }) }}</span>
                <NButton v-if="difficulties.length > 1" text type="error" size="tiny" @click="removeDifficulty(item.id)">
                  {{ t('common.remove') }}
                </NButton>
              </div>
              <NFormItem :label="t('generate.difficultyName')">
                <NInput v-model:value="item.name" maxlength="200" />
              </NFormItem>
              <NFormItem :label="t('generate.rating')">
                <div class="slider-field">
                  <NSlider v-model:value="item.difficulty" :min="1" :max="8" :step="0.1" />
                  <NInputNumber v-model:value="item.difficulty" :min="1" :max="8" :step="0.1" size="small" />
                </div>
              </NFormItem>
              <template v-if="isAdvanced">
                <NFormItem :label="t('generate.seed')">
                  <NInputNumber v-model:value="item.seed" :min="0" :max="65535" :step="1" />
                </NFormItem>
                <NFormItem :label="t('generate.descriptors')"><NDynamicTags v-model:value="item.descriptors" /></NFormItem>
                <NFormItem :label="t('generate.negative')"><NDynamicTags v-model:value="item.negativeDescriptors" /></NFormItem>
                <div class="two-column compact">
                  <NFormItem :label="t('generate.ar')"><NInputNumber v-model:value="item.approachRate" :min="0" :max="10" /></NFormItem>
                  <NFormItem :label="t('generate.cs')"><NInputNumber v-model:value="item.circleSize" :min="0" :max="10" /></NFormItem>
                </div>
                <div class="switch-row nested">
                  <div><strong>{{ t('generate.hitsounds') }}</strong><small>{{ t('generate.hitsoundsHelp') }}</small></div>
                  <NSwitch v-model:value="item.hitsounded" :aria-label="t('generate.hitsounds')" />
                </div>
              </template>
            </div>
          </div>
          <div class="switch-row">
            <div><strong>{{ t('generate.advanced') }}</strong><small>{{ t('generate.advancedHelp') }}</small></div>
            <NSwitch v-model:value="isAdvanced" :aria-label="t('generate.advanced')" />
          </div>
          <template v-if="isAdvanced">
            <NFormItem :label="t('generate.scenario')">
              <NSelect v-model:value="scenario" :options="scenarioOptions" />
            </NFormItem>
          </template>
        </NForm>
        <NAlert class="local-note" type="info" :show-icon="false"
          >{{ t('generate.local') }}</NAlert
        >
        <NButton type="primary" size="large" block :loading="creating" @click="createJob">
          <template #icon><NIcon :component="SparklesOutline" /></template>{{ t('generate.action') }}
        </NButton>
        <div class="estimate">
          <NIcon :component="PlayOutline" /><span>{{ t('generate.estimate') }}</span>
        </div>
      </NCard>
    </aside>
  </div>
</template>

<style scoped>
.generate-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.55fr) minmax(320px, 0.8fr);
  gap: 20px;
  align-items: start;
}
.generate-main {
  display: grid;
  gap: 20px;
}
.audio-dropzone {
  width: 100%;
  min-height: 190px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  border: 1px dashed rgba(127, 127, 127, 0.35);
  border-radius: 12px;
  color: inherit;
  background: rgba(127, 127, 127, 0.025);
  cursor: pointer;
  transition:
    border-color 0.2s,
    background 0.2s;
}
.audio-dropzone:hover,
.audio-dropzone:focus-visible {
  border-color: #ff66aa;
  background: rgba(255, 102, 170, 0.045);
  outline: none;
}
.upload-icon {
  width: 50px;
  height: 50px;
  margin: 0 auto 13px;
  display: grid;
  place-items: center;
  border-radius: 14px;
  color: #ff66aa;
  background: rgba(255, 102, 170, 0.13);
  font-size: 25px;
}
.audio-dropzone strong {
  font-size: 14px;
}
.audio-dropzone p {
  max-width: 80%;
  margin: 7px 0 0;
  overflow: hidden;
  color: #858da0;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.two-column {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}
.two-column.compact {
  gap: 10px;
}
.slider-field {
  display: grid;
  grid-template-columns: 1fr 82px;
  gap: 14px;
  align-items: center;
  width: 100%;
}
.difficulty-heading,
.difficulty-card-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.difficulty-heading {
  margin-bottom: 10px;
}
.difficulty-list {
  display: grid;
  gap: 10px;
}
.difficulty-card {
  padding: 12px;
  border: 1px solid rgba(127, 127, 127, 0.16);
  border-radius: 10px;
  background: rgba(127, 127, 127, 0.025);
}
.difficulty-card-heading {
  margin-bottom: 8px;
  font-size: 11px;
  font-weight: 700;
}
.switch-row.nested {
  padding-bottom: 2px;
}
.switch-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 13px 0;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.switch-row strong,
.switch-row small {
  display: block;
}
.switch-row strong {
  font-size: 12px;
}
.switch-row small {
  margin-top: 3px;
  color: #858da0;
  font-size: 10px;
}
.local-note {
  margin: 16px 0;
  font-size: 11px;
}
.estimate {
  display: flex;
  gap: 6px;
  justify-content: center;
  align-items: center;
  margin-top: 11px;
  color: #858da0;
  font-size: 10px;
}
.validation-alert {
  margin-top: 10px;
  font-size: 11px;
}
.reference-row {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.reference-row strong,
.reference-row small {
  display: block;
}
.reference-row strong {
  font-size: 12px;
}
.reference-row small {
  margin-top: 4px;
  color: #858da0;
  font-size: 10px;
}
@media (max-width: 1180px) {
  .generate-grid {
    grid-template-columns: 1fr 340px;
  }
}
</style>
