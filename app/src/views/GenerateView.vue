<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
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
  ImageOutline,
  MusicalNotesOutline,
  PlayOutline,
  SparklesOutline,
  VideocamOutline,
} from '@vicons/ionicons5'
import { useJobsStore } from '@/stores/jobs'
import { usePackagesStore } from '@/stores/packages'
import {
  authorizeBackgroundPreview,
  inspectBeatmapVideo,
  isDesktopRuntime,
  OFFICIAL_DESCRIPTOR_TAGS_URL,
  openOfficialDescriptorTags,
  selectAudioFile,
  selectBackgroundImage,
  selectBeatmapFile,
  selectVideoFile,
} from '@/services/desktop'
import { useI18n } from '@/i18n'
import { publicTags, validNumber, validTags, validText } from '@/utils/generateDraft'
import { capabilitiesForModel, supportedDifficulty } from '@/utils/modelCapabilities'
import type { ManagedPackage } from '@/types'

const router = useRouter()
const message = useMessage()
const jobsStore = useJobsStore()
const packageStore = usePackagesStore()
const { t } = useI18n()
const audioFile = ref('')
const title = ref('')
const artist = ref('')
const creator = ref('')
const titleUnicode = ref('')
const artistUnicode = ref('')
const songSource = ref('')
const tags = ref<string[]>([])
const model = ref('mapperatorinator-v32-standard')
const descriptorExamples = ['skillset/streams', 'skillset/jumps', 'style/clean']
const isAdvanced = ref(false)
const attempted = ref(false)
const creating = ref(false)
const scenario = ref('success')
const referenceBeatmap = ref('')
const backgroundImage = ref('')
const backgroundPreview = ref<{
  url: string
  width: number
  height: number
  sizeBytes: number
} | null>(null)
const videoFile = ref('')
const videoInfo = ref<{ sizeBytes: number; format: string } | null>(null)
const videoStartTimeMs = ref<number | null>(0)

interface EditableDifficulty {
  id: string
  name: string
  difficulty: number
  seed: number
  descriptors: string[]
  negativeDescriptors: string[]
  approachRate: number
  circleSize: number
  hpDrainRate: number | null
  overallDifficulty: number | null
  sliderMultiplier: number | null
  sliderTickRate: number | null
  hitsounded: boolean
}

let nextDifficultyId = 1
function makeDifficulty(rating = 4.8): EditableDifficulty {
  const id = nextDifficultyId++
  return {
    id: `difficulty-${id}`,
    name: `AI ${rating.toFixed(1)}s`,
    difficulty: rating,
    seed: Math.floor(Math.random() * 65_536),
    descriptors: [],
    negativeDescriptors: [],
    approachRate: 9,
    circleSize: 4,
    hpDrainRate: null,
    overallDifficulty: null,
    sliderMultiplier: null,
    sliderTickRate: null,
    hitsounded: true,
  }
}
const difficulties = ref<EditableDifficulty[]>([makeDifficulty()])
const selectedDifficultyId = ref(difficulties.value[0]!.id)
const selectedDifficulty = computed(
  () =>
    difficulties.value.find((item) => item.id === selectedDifficultyId.value) ??
    difficulties.value[0]!,
)

const desktop = isDesktopRuntime()
function modelStatusLabel(item: ManagedPackage) {
  if (
    item.status === 'installed' &&
    item.installedVersion === item.targetVersion &&
    item.installPath
  ) {
    return t('models.installed')
  }
  if (item.status === 'installed' && !item.installPath) return t('models.repair')
  if (item.status === 'update_available' || item.status === 'installed') return t('models.update')
  if (item.status === 'failed') return t('models.repair')
  if (item.status === 'downloaded') return t('generate.modelNeedsInstall')
  if (['queued', 'downloading', 'verifying', 'installing', 'removing'].includes(item.status)) {
    return t('generate.modelBusy')
  }
  return t('models.missing')
}
const modelOptions = computed(() =>
  desktop
    ? packageStore.models.map((item) => ({
        label: `${item.name} · ${modelStatusLabel(item)}`,
        value: item.id,
      }))
    : [{ label: 'Mapperatorinator V32 · osu!standard', value: 'mapperatorinator-v32-standard' }],
)
const selectedPackage = computed(() => packageStore.models.find((item) => item.id === model.value))
const modelCapabilities = computed(() => capabilitiesForModel(model.value))
const modelReady = computed(
  () =>
    !desktop ||
    (selectedPackage.value?.status === 'installed' &&
      selectedPackage.value.installedVersion === selectedPackage.value.targetVersion &&
      Boolean(selectedPackage.value.installPath)),
)
const modelStatus = computed(() => {
  if (!desktop) return t('common.ready')
  if (!selectedPackage.value) return t('common.checking')
  return modelStatusLabel(selectedPackage.value)
})
const modelCapabilityNotes = computed(() => {
  const capabilities = modelCapabilities.value
  if (!capabilities) return []
  return [
    capabilities.targetDifficulty
      ? t('generate.capabilityTarget')
      : t('generate.capabilityNoTarget'),
    capabilities.descriptors
      ? t('generate.capabilityDescriptors')
      : t('generate.capabilityNoDescriptors'),
    capabilities.hitsounded
      ? t('generate.capabilityHitsounds')
      : t('generate.capabilityNoHitsounds'),
    capabilities.referenceTiming
      ? t('generate.capabilityReference')
      : t('generate.capabilityNoReference'),
  ]
})
const scenarioOptions = [
  { label: 'Production inference', value: 'success' },
  { label: 'Simulate generation failure', value: 'failure' },
  { label: 'Simulate CUDA out of memory', value: 'oom' },
  { label: 'Simulate worker timeout', value: 'hang' },
  { label: 'Simulate unexpected process exit', value: 'crash' },
  { label: 'Simulate malformed protocol message', value: 'malformed' },
]
const validPublicTags = computed(() => validTags(tags.value))
const validV020 = computed(
  () =>
    [creator.value, titleUnicode.value, artistUnicode.value, songSource.value].every((value) =>
      validText(value),
    ) &&
    validPublicTags.value &&
    difficulties.value.every(
      (item) =>
        validNumber(item.hpDrainRate, 0, 10) &&
        validNumber(item.overallDifficulty, 0, 10) &&
        validNumber(item.sliderMultiplier, 0.1, 10) &&
        validNumber(item.sliderTickRate, 0.1, 10),
    ),
)
const effectivePublicTags = computed(() => publicTags(tags.value))
const canGenerate = computed(() =>
  Boolean(
    audioFile.value &&
    modelCapabilities.value &&
    modelReady.value &&
    validText(title.value, true) &&
    validText(artist.value, true) &&
    validV020.value &&
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
  const difficulty = makeDifficulty(Math.min(8, Number((previous + 0.8).toFixed(1))))
  if (!modelCapabilities.value?.targetDifficulty)
    difficulty.name = `AI ${difficulties.value.length + 1}`
  difficulties.value.push(difficulty)
  selectedDifficultyId.value = difficulty.id
}

function removeDifficulty(id: string) {
  if (difficulties.value.length > 1) {
    const removedIndex = difficulties.value.findIndex((item) => item.id === id)
    difficulties.value = difficulties.value.filter((item) => item.id !== id)
    if (selectedDifficultyId.value === id) {
      selectedDifficultyId.value =
        difficulties.value[Math.min(removedIndex, difficulties.value.length - 1)]!.id
    }
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

async function openDescriptorList(event: MouseEvent) {
  if (!desktop) return
  event.preventDefault()
  try {
    await openOfficialDescriptorTags()
  } catch {
    message.error(t('generate.descriptorLinkFailed'))
  }
}

async function copyDescriptorExample(tag: string) {
  try {
    await navigator.clipboard.writeText(tag)
    message.success(t('generate.descriptorCopied', { tag }))
  } catch {
    message.error(t('generate.descriptorCopyFailed'))
  }
}

async function chooseReferenceBeatmap() {
  if (!isDesktopRuntime()) {
    message.info(t('generate.pickDesktop'))
    return
  }
  const selected = await selectBeatmapFile()
  if (selected) referenceBeatmap.value = selected
}

async function chooseBackgroundImage() {
  if (!isDesktopRuntime()) {
    message.info(t('generate.pickDesktop'))
    return
  }
  const selected = await selectBackgroundImage()
  if (!selected) return
  try {
    const preview = await authorizeBackgroundPreview(selected)
    backgroundImage.value = preview.path
    backgroundPreview.value = preview
  } catch (reason) {
    message.error(t('generate.backgroundFailed', { error: String(reason) }))
  }
}

function clearBackgroundImage() {
  backgroundImage.value = ''
  backgroundPreview.value = null
}

async function chooseVideo() {
  if (!isDesktopRuntime()) {
    message.info(t('generate.pickDesktop'))
    return
  }
  const selected = await selectVideoFile()
  if (!selected) return
  try {
    const info = await inspectBeatmapVideo(selected)
    videoFile.value = info.path
    videoInfo.value = info
  } catch (reason) {
    message.error(t('generate.videoFailed', { error: String(reason) }))
  }
}

function clearVideo() {
  videoFile.value = ''
  videoInfo.value = null
  videoStartTimeMs.value = 0
}

async function createJob() {
  attempted.value = true
  if (!modelCapabilities.value || !modelReady.value) {
    message.warning(t('generate.modelRequired'))
    return
  }
  if (!validV020.value) {
    if (
      difficulties.value.some(
        (item) =>
          !validNumber(item.hpDrainRate, 0, 10) ||
          !validNumber(item.overallDifficulty, 0, 10) ||
          !validNumber(item.sliderMultiplier, 0.1, 10) ||
          !validNumber(item.sliderTickRate, 0.1, 10),
      )
    ) {
      isAdvanced.value = true
    }
    message.warning(t('generate.invalidFields'))
    return
  }
  if (!canGenerate.value) {
    message.warning(t('generate.missing'))
    return
  }
  creating.value = true
  try {
    const primary = difficulties.value[0]!
    const capabilities = modelCapabilities.value
    const requestDifficulties = difficulties.value.map((item) =>
      supportedDifficulty({ ...item, name: item.name.trim() }, capabilities),
    )
    const requestPrimary = requestDifficulties[0]!
    await jobsStore.createJob({
      requestSchemaVersion: 2,
      title: title.value.trim(),
      artist: artist.value.trim(),
      creator: creator.value.trim() || null,
      titleUnicode: titleUnicode.value.trim() || null,
      artistUnicode: artistUnicode.value.trim() || null,
      source: songSource.value.trim() || null,
      tags: tags.value.map((tag) => tag.trim()),
      audioFile: audioFile.value,
      model: model.value,
      modelVersion: selectedPackage.value?.targetVersion,
      modelSource: selectedPackage.value?.sourceUrl,
      difficulty: requestPrimary.difficulty,
      difficulties: requestDifficulties,
      gamemode: 0,
      seed: primary.seed,
      descriptors: requestPrimary.descriptors,
      negativeDescriptors: requestPrimary.negativeDescriptors,
      referenceBeatmap: capabilities.referenceTiming
        ? referenceBeatmap.value || undefined
        : undefined,
      backgroundImage: backgroundImage.value || undefined,
      videoFile: videoFile.value || undefined,
      videoStartTimeMs: videoStartTimeMs.value ?? 0,
      approachRate: primary.approachRate,
      circleSize: primary.circleSize,
      hitsounded: requestPrimary.hitsounded,
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

onMounted(() => {
  void packageStore.initialize().catch((reason) => {
    message.error(String(reason))
  })
})

watch(model, () => {
  if (modelCapabilities.value?.targetDifficulty) return
  difficulties.value.forEach((item, index) => {
    if (/^AI \d(?:\.\d)?s$/.test(item.name)) item.name = `AI ${index + 1}`
  })
})
</script>

<template>
  <div class="generate-layout">
    <NCard class="panel-card source-card" :bordered="false">
      <div class="card-heading">
        <div>
          <h2 class="section-title">{{ t('generate.songAndAssets') }}</h2>
          <p class="section-copy">{{ t('generate.songAndAssetsHelp') }}</p>
        </div>
        <NTag v-if="audioFile" size="small" type="success">{{ t('generate.audioReady') }}</NTag>
      </div>

      <div class="source-grid">
        <div>
          <button class="audio-dropzone" type="button" @click="chooseAudio">
            <div class="upload-icon">
              <NIcon :component="audioFile ? MusicalNotesOutline : CloudUploadOutline" />
            </div>
            <span>
              <strong>{{ audioDisplayName || t('generate.chooseAudio') }}</strong>
              <small>{{ audioFile || t('generate.audioTypes') }}</small>
            </span>
          </button>
          <NAlert
            v-if="attempted && !audioFile"
            class="validation-alert"
            type="error"
            :show-icon="false"
            >{{ t('generate.audioRequired') }}</NAlert
          >
        </div>

        <NForm class="metadata-form" label-placement="top">
          <div class="two-column">
            <NFormItem
              :label="t('generate.title')"
              required
              :validation-status="attempted && !validText(title, true) ? 'error' : undefined"
              :feedback="
                attempted && !validText(title, true) ? t('generate.enterTitle') : undefined
              "
            >
              <NInput v-model:value="title" :placeholder="t('generate.songTitle')" />
            </NFormItem>
            <NFormItem
              :label="t('generate.artist')"
              required
              :validation-status="attempted && !validText(artist, true) ? 'error' : undefined"
              :feedback="
                attempted && !validText(artist, true) ? t('generate.enterArtist') : undefined
              "
            >
              <NInput v-model:value="artist" :placeholder="t('generate.artistName')" />
            </NFormItem>
          </div>
          <div class="two-column">
            <NFormItem
              :label="t('generate.creator')"
              :validation-status="attempted && !validText(creator) ? 'error' : undefined"
              :feedback="attempted && !validText(creator) ? t('generate.textInvalid') : undefined"
            >
              <NInput
                v-model:value="creator"
                :placeholder="t('generate.creatorDefault')"
                maxlength="200"
              />
            </NFormItem>
            <NFormItem
              :label="t('generate.songSource')"
              :validation-status="attempted && !validText(songSource) ? 'error' : undefined"
              :feedback="
                attempted && !validText(songSource) ? t('generate.textInvalid') : undefined
              "
            >
              <NInput
                v-model:value="songSource"
                :placeholder="t('generate.optional')"
                maxlength="200"
              />
            </NFormItem>
          </div>
          <NAlert class="metadata-note" type="warning" :show-icon="false">{{
            t('generate.creatorDisclosure')
          }}</NAlert>
          <div class="two-column">
            <NFormItem
              :label="t('generate.titleUnicode')"
              :validation-status="attempted && !validText(titleUnicode) ? 'error' : undefined"
              :feedback="
                attempted && !validText(titleUnicode) ? t('generate.textInvalid') : undefined
              "
            >
              <NInput
                v-model:value="titleUnicode"
                :placeholder="t('generate.sameAsTitle')"
                maxlength="200"
              />
            </NFormItem>
            <NFormItem
              :label="t('generate.artistUnicode')"
              :validation-status="attempted && !validText(artistUnicode) ? 'error' : undefined"
              :feedback="
                attempted && !validText(artistUnicode) ? t('generate.textInvalid') : undefined
              "
            >
              <NInput
                v-model:value="artistUnicode"
                :placeholder="t('generate.sameAsArtist')"
                maxlength="200"
              />
            </NFormItem>
          </div>
          <NFormItem
            :label="t('generate.publicTags')"
            :validation-status="attempted && !validPublicTags ? 'error' : undefined"
            :feedback="attempted && !validPublicTags ? t('generate.tagsInvalid') : undefined"
          >
            <NDynamicTags v-model:value="tags" />
          </NFormItem>
          <p class="field-help">{{ t('generate.tagsHelp') }}</p>
        </NForm>
      </div>

      <div class="asset-grid">
        <div v-if="modelCapabilities?.referenceTiming" class="asset-item">
          <div class="asset-copy">
            <NIcon :component="DocumentAttachOutline" />
            <span
              ><strong>{{ t('generate.reference') }}</strong
              ><small>{{ referenceDisplayName || t('generate.referenceHelp') }}</small></span
            >
          </div>
          <NButton secondary size="small" @click="chooseReferenceBeatmap">
            {{ referenceBeatmap ? t('common.change') : t('generate.chooseOsu') }}
          </NButton>
        </div>

        <div class="asset-item">
          <div class="asset-copy">
            <img
              v-if="backgroundPreview"
              :src="backgroundPreview.url"
              :alt="t('generate.backgroundPreview')"
            />
            <NIcon v-else :component="ImageOutline" />
            <span>
              <strong>{{ t('generate.background') }}</strong>
              <small v-if="backgroundPreview"
                >{{ backgroundPreview.width }}×{{ backgroundPreview.height }} ·
                {{ (backgroundPreview.sizeBytes / 1024 / 1024).toFixed(1) }} MB</small
              >
              <small v-else>{{ t('generate.backgroundHelp') }}</small>
            </span>
          </div>
          <div class="asset-actions">
            <NButton v-if="backgroundImage" text size="tiny" @click="clearBackgroundImage">{{
              t('common.remove')
            }}</NButton>
            <NButton secondary size="small" @click="chooseBackgroundImage">{{
              backgroundImage ? t('common.change') : t('generate.chooseImage')
            }}</NButton>
          </div>
        </div>

        <div class="asset-item video-asset">
          <div class="asset-copy">
            <NIcon :component="VideocamOutline" />
            <span>
              <strong>{{ t('generate.video') }}</strong>
              <small v-if="videoInfo"
                >{{ videoFile.split(/[\\/]/).at(-1) }} · {{ videoInfo.format.toUpperCase() }} ·
                {{ (videoInfo.sizeBytes / 1024 / 1024).toFixed(1) }} MB</small
              >
              <small v-else>{{ t('generate.videoHelp') }}</small>
            </span>
          </div>
          <div class="asset-actions">
            <NInputNumber
              v-if="videoFile"
              v-model:value="videoStartTimeMs"
              class="video-start-input"
              :min="0"
              :max="3600000"
              :step="100"
              :precision="0"
              :aria-label="t('generate.videoStartTime')"
              ><template #suffix>ms</template></NInputNumber
            >
            <NButton v-if="videoFile" text size="tiny" @click="clearVideo">{{
              t('common.remove')
            }}</NButton>
            <NButton secondary size="small" @click="chooseVideo">{{
              videoFile ? t('common.change') : t('generate.chooseVideo')
            }}</NButton>
          </div>
        </div>
      </div>
    </NCard>

    <NCard class="panel-card setup-card" :bordered="false">
      <div class="setup-heading">
        <div>
          <h2 class="section-title">{{ t('generate.setup') }}</h2>
          <p class="section-copy">{{ t('generate.setupHelp') }}</p>
        </div>
        <div class="model-field">
          <span>{{ t('generate.model') }}</span>
          <NSelect
            v-model:value="model"
            :options="modelOptions"
            :aria-label="t('generate.model')"
          />
        </div>
      </div>

      <div class="model-details" aria-live="polite">
        <div class="model-details-heading">
          <strong>{{ selectedPackage?.name ?? t('generate.model') }}</strong>
          <NTag size="small" :type="modelReady ? 'success' : 'warning'">{{ modelStatus }}</NTag>
          <NTag v-if="selectedPackage" size="small">v{{ selectedPackage.targetVersion }}</NTag>
        </div>
        <p>{{ t('generate.modelCapabilities') }}</p>
        <ul>
          <li v-for="note in modelCapabilityNotes" :key="note">{{ note }}</li>
        </ul>
        <NAlert v-if="desktop && !modelReady" type="warning" :show-icon="false">
          {{ t('generate.modelRequired') }}
          <NButton text type="primary" @click="router.push('/models')">{{
            t('generate.manageModels')
          }}</NButton>
        </NAlert>
        <p v-if="selectedPackage?.acquisition === 'manual_import'" class="model-caveat">
          {{ t('generate.manualModelCaveat') }}
        </p>
      </div>

      <div class="difficulty-workspace">
        <aside class="difficulty-rail">
          <div class="difficulty-heading">
            <div>
              <strong>{{ t('generate.difficulties') }}</strong
              ><small>{{ t('generate.difficultyListHelp') }}</small>
            </div>
            <NButton
              size="small"
              secondary
              :disabled="difficulties.length >= 8"
              @click="addDifficulty"
              >{{ t('generate.addDifficulty') }}</NButton
            >
          </div>
          <div class="difficulty-list">
            <div
              v-for="(item, index) in difficulties"
              :key="item.id"
              class="difficulty-list-item"
              :class="{ selected: item.id === selectedDifficultyId }"
            >
              <button
                class="difficulty-select"
                type="button"
                @click="selectedDifficultyId = item.id"
              >
                <span>{{
                  item.name || t('generate.difficultyNumber', { number: index + 1 })
                }}</span>
                <small
                  >{{
                    modelCapabilities?.targetDifficulty ? `${item.difficulty.toFixed(1)}★ · ` : ''
                  }}AR {{ item.approachRate }} · CS {{ item.circleSize }} · Seed
                  {{ item.seed }}</small
                >
              </button>
              <NButton
                v-if="difficulties.length > 1"
                class="difficulty-remove"
                text
                type="error"
                size="tiny"
                :aria-label="`${t('common.remove')} ${item.name}`"
                @click="removeDifficulty(item.id)"
                >×</NButton
              >
            </div>
          </div>
        </aside>

        <section class="difficulty-editor">
          <div class="editor-heading">
            <div>
              <span>{{ t('generate.editing') }}</span
              ><strong>{{ selectedDifficulty.name }}</strong>
            </div>
          </div>
          <NForm label-placement="top">
            <div class="two-column editor-basics">
              <NFormItem :label="t('generate.difficultyName')">
                <NInput v-model:value="selectedDifficulty.name" maxlength="200" />
              </NFormItem>
              <NFormItem v-if="modelCapabilities?.targetDifficulty" :label="t('generate.rating')">
                <div class="slider-field">
                  <NSlider
                    v-model:value="selectedDifficulty.difficulty"
                    :min="1"
                    :max="8"
                    :step="0.1"
                  />
                  <NInputNumber
                    v-model:value="selectedDifficulty.difficulty"
                    :min="1"
                    :max="8"
                    :step="0.1"
                    size="small"
                  />
                </div>
              </NFormItem>
              <NAlert v-else type="info" :show-icon="false" class="no-target-note">
                {{ t('generate.noTargetHelp') }}
              </NAlert>
            </div>
            <div v-if="modelCapabilities?.descriptors" class="two-column descriptor-grid">
              <NFormItem :label="t('generate.descriptors')"
                ><NDynamicTags v-model:value="selectedDifficulty.descriptors"
              /></NFormItem>
              <NFormItem :label="t('generate.negative')" :feedback="t('generate.negativeHelp')"
                ><NDynamicTags v-model:value="selectedDifficulty.negativeDescriptors"
              /></NFormItem>
            </div>
            <div v-if="modelCapabilities?.descriptors" class="descriptor-guide">
              <p>{{ t('generate.descriptorGuide') }}</p>
              <a
                :href="OFFICIAL_DESCRIPTOR_TAGS_URL"
                target="_blank"
                rel="noopener noreferrer"
                @click="openDescriptorList"
                >{{ t('generate.descriptorList') }}</a
              >
              <div class="descriptor-examples">
                <span>{{ t('generate.descriptorExamples') }}</span>
                <NButton
                  v-for="tag in descriptorExamples"
                  :key="tag"
                  size="tiny"
                  secondary
                  :aria-label="t('generate.copyDescriptor', { tag })"
                  @click="copyDescriptorExample(tag)"
                  ><code>{{ tag }}</code></NButton
                >
              </div>
            </div>

            <button
              class="advanced-toggle"
              type="button"
              :aria-expanded="isAdvanced"
              @click="isAdvanced = !isAdvanced"
            >
              <span
                ><strong>{{ t('generate.advanced') }}</strong
                ><small>{{
                  t('generate.advancedSummary', {
                    seed: selectedDifficulty.seed,
                    ar: selectedDifficulty.approachRate,
                    cs: selectedDifficulty.circleSize,
                    hitsounds:
                      modelCapabilities?.hitsounded && selectedDifficulty.hitsounded
                        ? t('common.on')
                        : t('common.off'),
                  })
                }}</small></span
              >
              <span class="advanced-chevron" :class="{ open: isAdvanced }">⌄</span>
            </button>

            <div v-if="isAdvanced" class="advanced-panel">
              <div class="advanced-grid">
                <NFormItem :label="t('generate.seed')"
                  ><NInputNumber
                    v-model:value="selectedDifficulty.seed"
                    :min="0"
                    :max="65535"
                    :step="1"
                /></NFormItem>
                <NFormItem :label="t('generate.ar')"
                  ><NInputNumber
                    v-model:value="selectedDifficulty.approachRate"
                    :min="0"
                    :max="10"
                    :step="0.1"
                /></NFormItem>
                <NFormItem :label="t('generate.cs')"
                  ><NInputNumber
                    v-model:value="selectedDifficulty.circleSize"
                    :min="0"
                    :max="10"
                    :step="0.1"
                /></NFormItem>
                <NFormItem
                  :label="t('generate.hp')"
                  :validation-status="
                    attempted && !validNumber(selectedDifficulty.hpDrainRate, 0, 10)
                      ? 'error'
                      : undefined
                  "
                  :feedback="
                    attempted && !validNumber(selectedDifficulty.hpDrainRate, 0, 10)
                      ? t('generate.numberInvalid', { min: 0, max: 10 })
                      : undefined
                  "
                >
                  <NInputNumber
                    v-model:value="selectedDifficulty.hpDrainRate"
                    :min="0"
                    :max="10"
                    :step="0.1"
                    :placeholder="t('generate.inheritDefault')"
                    :aria-label="t('generate.hp')"
                  />
                </NFormItem>
                <NFormItem
                  :label="t('generate.od')"
                  :validation-status="
                    attempted && !validNumber(selectedDifficulty.overallDifficulty, 0, 10)
                      ? 'error'
                      : undefined
                  "
                  :feedback="
                    attempted && !validNumber(selectedDifficulty.overallDifficulty, 0, 10)
                      ? t('generate.numberInvalid', { min: 0, max: 10 })
                      : undefined
                  "
                >
                  <NInputNumber
                    v-model:value="selectedDifficulty.overallDifficulty"
                    :min="0"
                    :max="10"
                    :step="0.1"
                    :placeholder="t('generate.inheritDefault')"
                    :aria-label="t('generate.od')"
                  />
                </NFormItem>
                <NFormItem
                  :label="t('generate.sliderMultiplier')"
                  :validation-status="
                    attempted && !validNumber(selectedDifficulty.sliderMultiplier, 0.1, 10)
                      ? 'error'
                      : undefined
                  "
                  :feedback="
                    attempted && !validNumber(selectedDifficulty.sliderMultiplier, 0.1, 10)
                      ? t('generate.numberInvalid', { min: 0.1, max: 10 })
                      : undefined
                  "
                >
                  <NInputNumber
                    v-model:value="selectedDifficulty.sliderMultiplier"
                    :min="0.1"
                    :max="10"
                    :step="0.1"
                    :placeholder="t('generate.inheritDefault')"
                    :aria-label="t('generate.sliderMultiplier')"
                  />
                </NFormItem>
                <NFormItem
                  :label="t('generate.sliderTickRate')"
                  :validation-status="
                    attempted && !validNumber(selectedDifficulty.sliderTickRate, 0.1, 10)
                      ? 'error'
                      : undefined
                  "
                  :feedback="
                    attempted && !validNumber(selectedDifficulty.sliderTickRate, 0.1, 10)
                      ? t('generate.numberInvalid', { min: 0.1, max: 10 })
                      : undefined
                  "
                >
                  <NInputNumber
                    v-model:value="selectedDifficulty.sliderTickRate"
                    :min="0.1"
                    :max="10"
                    :step="0.1"
                    :placeholder="t('generate.inheritDefault')"
                    :aria-label="t('generate.sliderTickRate')"
                  />
                </NFormItem>
                <div v-if="modelCapabilities?.hitsounded" class="switch-row nested">
                  <div>
                    <strong>{{ t('generate.hitsounds') }}</strong
                    ><small>{{ t('generate.hitsoundsHelp') }}</small>
                  </div>
                  <NSwitch
                    v-model:value="selectedDifficulty.hitsounded"
                    :aria-label="t('generate.hitsounds')"
                  />
                </div>
              </div>
              <NFormItem class="scenario-field" :label="t('generate.scenario')"
                ><NSelect v-model:value="scenario" :options="scenarioOptions"
              /></NFormItem>
            </div>
          </NForm>
        </section>
      </div>

      <NAlert class="local-note" type="info" :show-icon="false">{{ t('generate.local') }}</NAlert>
      <div class="request-summary" aria-live="polite">
        <strong>{{ t('generate.requestSummary') }}</strong>
        <p>
          {{ title || t('generate.songTitle') }} — {{ artist || t('generate.artistName') }} ·
          {{ t('generate.creator') }}: {{ creator.trim() || 'Beatforge AI' }} ·
          {{ t('generate.songSource') }}: {{ songSource.trim() || '—' }}
        </p>
        <p>
          {{ t('generate.titleUnicode') }}: {{ titleUnicode.trim() || title || '—' }} ·
          {{ t('generate.artistUnicode') }}: {{ artistUnicode.trim() || artist || '—' }} ·
          {{ t('generate.publicTags') }}: {{ effectivePublicTags }}
        </p>
        <ul>
          <li v-for="item in difficulties" :key="item.id">
            {{ item.name }} ·
            {{ modelCapabilities?.targetDifficulty ? `${item.difficulty.toFixed(1)}★ · ` : '' }}AR
            {{ item.approachRate }} · CS {{ item.circleSize }} · HP {{ item.hpDrainRate ?? '—' }} ·
            OD {{ item.overallDifficulty ?? '—' }} · SV {{ item.sliderMultiplier ?? '—' }} · Tick
            {{ item.sliderTickRate ?? '—' }}
          </li>
        </ul>
        <small>{{ t('generate.summaryFallback') }}</small>
      </div>
      <div class="generate-footer">
        <div class="generation-summary">
          <strong>{{ t('generate.readySummary', { count: difficulties.length }) }}</strong>
          <span><NIcon :component="PlayOutline" />{{ t('generate.estimate') }}</span>
        </div>
        <NButton
          type="primary"
          size="large"
          :loading="creating"
          :disabled="desktop && !modelReady"
          @click="createJob"
        >
          <template #icon><NIcon :component="SparklesOutline" /></template
          >{{ t('generate.actionCount', { count: difficulties.length }) }}
        </NButton>
      </div>
    </NCard>
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
.background-row {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.background-row strong,
.background-row small {
  display: block;
}
.background-row small {
  margin-top: 4px;
  color: #858da0;
  font-size: 10px;
}
.background-preview,
.background-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}
.background-preview img {
  width: 96px;
  height: 54px;
  object-fit: cover;
  border-radius: 7px;
}
.video-row {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.video-row strong,
.video-row small {
  display: block;
}
.video-row small {
  margin-top: 4px;
  color: #858da0;
  font-size: 10px;
}
.video-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
  align-items: center;
}
.video-time small {
  margin: 0 0 3px;
}
.video-start-input {
  width: 160px;
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

.generate-layout {
  display: grid;
  gap: 20px;
}
.source-grid {
  display: grid;
  grid-template-columns: minmax(300px, 0.82fr) minmax(0, 1.18fr);
  gap: 18px;
  align-items: start;
}
.source-card .audio-dropzone {
  min-height: 116px;
  padding: 18px;
  flex-direction: row;
  justify-content: flex-start;
  gap: 14px;
  text-align: left;
}
.source-card .upload-icon {
  flex: 0 0 44px;
  width: 44px;
  height: 44px;
  margin: 0;
}
.audio-dropzone span,
.audio-dropzone small {
  min-width: 0;
  display: block;
}
.audio-dropzone small {
  max-width: 100%;
  margin-top: 5px;
  overflow: hidden;
  color: #858da0;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.metadata-form :deep(.n-form-item) {
  margin-bottom: 10px;
}
.metadata-note {
  margin: 0 0 14px;
  font-size: 11px;
}
.field-help {
  margin: -6px 0 12px;
  color: #929bad;
  font-size: 11px;
  line-height: 1.5;
}
.asset-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin-top: 4px;
  padding-top: 14px;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.asset-item {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
  border: 1px solid rgba(127, 127, 127, 0.14);
  border-radius: 10px;
  background: rgba(127, 127, 127, 0.025);
}
.asset-copy {
  min-width: 0;
  display: flex;
  gap: 9px;
  align-items: center;
}
.asset-copy > .n-icon {
  flex: 0 0 auto;
  color: #ff66aa;
  font-size: 19px;
}
.asset-copy img {
  flex: 0 0 auto;
  width: 54px;
  height: 34px;
  object-fit: cover;
  border-radius: 6px;
}
.asset-copy span,
.asset-copy strong,
.asset-copy small {
  min-width: 0;
  display: block;
}
.asset-copy strong {
  font-size: 12px;
}
.asset-copy small {
  margin-top: 3px;
  overflow: hidden;
  color: #858da0;
  font-size: 9px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.asset-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  align-items: center;
}
.video-start-input {
  width: 126px;
}
.setup-heading {
  display: flex;
  justify-content: space-between;
  gap: 24px;
  align-items: flex-start;
  margin-bottom: 16px;
}
.model-field {
  flex: 0 1 420px;
  display: grid;
  grid-template-columns: auto minmax(240px, 1fr);
  gap: 10px;
  align-items: center;
}
.model-field > span {
  color: #9aa2b2;
  font-size: 11px;
  font-weight: 700;
}
.model-details {
  padding: 12px 14px;
  margin-bottom: 16px;
  border: 1px solid rgba(127, 127, 127, 0.18);
  border-radius: 12px;
  background: rgba(127, 127, 127, 0.035);
}
.model-details-heading {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 12px;
}
.model-details p {
  margin: 8px 0 5px;
  color: #858da0;
  font-size: 10px;
}
.model-details ul {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 16px;
  padding: 0;
  margin: 0 0 9px;
  list-style: none;
  font-size: 10px;
}
.model-details li::before {
  content: '•';
  margin-right: 5px;
  color: #ff66aa;
}
.model-details .model-caveat {
  margin-bottom: 0;
}
.no-target-note {
  align-self: center;
}
.difficulty-workspace {
  display: grid;
  grid-template-columns: minmax(250px, 0.62fr) minmax(0, 1.65fr);
  gap: 18px;
  align-items: stretch;
}
.difficulty-rail,
.difficulty-editor {
  padding: 16px;
  border: 1px solid rgba(127, 127, 127, 0.16);
  border-radius: 14px;
  background: rgba(127, 127, 127, 0.025);
}
.difficulty-heading {
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 12px;
}
.difficulty-heading strong,
.difficulty-heading small {
  display: block;
}
.difficulty-heading strong {
  font-size: 12px;
}
.difficulty-heading small {
  margin-top: 3px;
  color: #858da0;
  font-size: 9px;
}
.difficulty-list-item {
  position: relative;
  display: flex;
  align-items: stretch;
  border: 1px solid rgba(127, 127, 127, 0.16);
  border-radius: 10px;
  background: rgba(127, 127, 127, 0.035);
  overflow: hidden;
  transition:
    border-color 0.16s,
    background 0.16s;
}
.difficulty-list-item:hover {
  border-color: rgba(255, 102, 170, 0.45);
}
.difficulty-list-item.selected {
  border-color: #ff66aa;
  background: rgba(255, 102, 170, 0.09);
}
.difficulty-select {
  min-width: 0;
  flex: 1;
  padding: 13px 36px 13px 14px;
  border: 0;
  color: inherit;
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.difficulty-select:focus-visible {
  outline: 2px solid #ff66aa;
  outline-offset: -2px;
}
.difficulty-select span,
.difficulty-select small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.difficulty-select span {
  font-size: 12px;
  font-weight: 700;
}
.difficulty-select small {
  margin-top: 5px;
  color: #929bad;
  font-size: 9px;
}
.difficulty-remove {
  position: absolute;
  top: 7px;
  right: 8px;
  z-index: 1;
}
.editor-heading {
  margin-bottom: 14px;
}
.editor-heading span,
.editor-heading strong {
  display: inline;
}
.editor-heading span {
  margin-right: 7px;
  color: #858da0;
  font-size: 11px;
}
.editor-heading strong {
  font-size: 14px;
}
.editor-basics,
.descriptor-grid {
  gap: 16px;
}
.descriptor-guide {
  padding: 10px 12px;
  margin: -3px 0 14px;
  border: 1px solid rgba(127, 127, 127, 0.18);
  border-radius: 9px;
  font-size: 10px;
}
.descriptor-guide p {
  margin: 0 0 7px;
  color: #9aa2b2;
}
.descriptor-guide a {
  color: #ff66aa;
  text-decoration: underline;
  text-underline-offset: 2px;
}
.descriptor-guide a:focus-visible {
  outline: 2px solid #ff66aa;
  outline-offset: 3px;
}
.descriptor-examples {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}
.descriptor-examples code {
  font-size: 10px;
}
.difficulty-editor :deep(.n-form-item) {
  margin-bottom: 12px;
}
.advanced-toggle {
  width: 100%;
  display: flex;
  justify-content: space-between;
  gap: 14px;
  align-items: center;
  padding: 12px 14px;
  border: 1px solid rgba(255, 102, 170, 0.55);
  border-radius: 9px;
  color: inherit;
  background: rgba(127, 127, 127, 0.035);
  text-align: left;
  cursor: pointer;
}
.advanced-toggle:hover,
.advanced-toggle:focus-visible {
  border-color: #ff66aa;
  outline: none;
}
.advanced-toggle strong,
.advanced-toggle small {
  display: inline;
}
.advanced-toggle strong {
  margin-right: 12px;
  font-size: 12px;
}
.advanced-toggle small {
  color: #929bad;
  font-size: 10px;
}
.advanced-chevron {
  color: #ff66aa;
  font-size: 18px;
  line-height: 1;
  transform: rotate(0deg);
  transition: transform 0.16s;
}
.advanced-chevron.open {
  transform: rotate(180deg);
}
.advanced-panel {
  margin-top: 10px;
  padding: 14px;
  border: 1px dashed rgba(127, 127, 127, 0.2);
  border-radius: 9px;
  background: rgba(0, 0, 0, 0.06);
}
.advanced-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
  align-items: end;
}
.advanced-grid .switch-row {
  min-height: 58px;
  padding: 0 2px 12px;
  border-top: 0;
}
.scenario-field {
  max-width: 420px;
  margin-top: 2px;
}
.local-note {
  margin: 14px 0 0;
}
.request-summary {
  margin-top: 14px;
  padding: 14px 16px;
  border: 1px solid rgba(127, 127, 127, 0.14);
  border-radius: 11px;
  background: rgba(127, 127, 127, 0.025);
  overflow-wrap: anywhere;
}
.request-summary strong {
  display: block;
  margin-bottom: 6px;
  font-size: 12px;
}
.request-summary p,
.request-summary li,
.request-summary small {
  color: #aeb6c4;
  font-size: 11px;
  line-height: 1.55;
}
.request-summary p {
  margin: 4px 0;
}
.request-summary ul {
  margin: 8px 0 4px;
  padding-left: 18px;
}
.request-summary small {
  color: #929bad;
}
.generate-footer {
  display: flex;
  justify-content: space-between;
  gap: 18px;
  align-items: center;
  margin-top: 14px;
  padding: 14px 16px;
  border: 1px solid rgba(127, 127, 127, 0.14);
  border-radius: 11px;
  background: rgba(127, 127, 127, 0.025);
}
.generate-footer .n-button {
  min-width: 220px;
}
.generation-summary strong,
.generation-summary span {
  display: block;
}
.generation-summary strong {
  font-size: 12px;
}
.generation-summary span {
  margin-top: 4px;
  color: #858da0;
  font-size: 10px;
}
.generation-summary .n-icon {
  margin-right: 5px;
  vertical-align: -2px;
}
@media (max-width: 1180px) {
  .asset-grid {
    grid-template-columns: 1fr;
  }
  .asset-item {
    flex-direction: row;
    align-items: center;
  }
  .advanced-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
@media (max-width: 900px) {
  .source-grid,
  .difficulty-workspace {
    grid-template-columns: 1fr;
  }
  .setup-heading,
  .generate-footer {
    align-items: stretch;
    flex-direction: column;
  }
  .model-field {
    flex-basis: auto;
  }
  .generate-footer .n-button {
    width: 100%;
  }
}
@media (max-width: 640px) {
  .two-column,
  .advanced-grid {
    grid-template-columns: 1fr;
  }
  .model-field {
    grid-template-columns: 1fr;
  }
  .asset-item {
    align-items: stretch;
    flex-direction: column;
  }
  .advanced-toggle strong,
  .advanced-toggle small {
    display: block;
  }
  .advanced-toggle small {
    margin-top: 4px;
  }
}
</style>
