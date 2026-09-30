<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  NAlert,
  NButton,
  NCard,
  NForm,
  NFormItem,
  NIcon,
  NInput,
  NInputNumber,
  NRadioButton,
  NRadioGroup,
  NSelect,
  NSwitch,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  CheckmarkCircleOutline,
  FolderOpenOutline,
  HardwareChipOutline,
  RefreshOutline,
  SaveOutline,
} from '@vicons/ionicons5'
import {
  isDesktopRuntime,
  loadDesktopSettings,
  saveDesktopSettings,
  selectDirectory,
} from '@/services/desktop'
import { useDesktopStore } from '@/stores/desktop'

const message = useMessage()
const desktopStore = useDesktopStore()
const savedSettings = JSON.parse(localStorage.getItem('beatforge:settings') ?? '{}')
const outputPath = ref(savedSettings.outputPath ?? '')
const tempPath = ref(savedSettings.tempPath ?? '')
const osuPath = ref(savedSettings.osuPath ?? '')
const device = ref(savedSettings.device ?? 'cuda')
const precision = ref(savedSettings.precision ?? 'fp16')
const workers = ref(savedSettings.workers ?? 2)
const autoImport = ref(savedSettings.autoImport ?? true)
const keepIntermediate = ref(savedSettings.keepIntermediate ?? false)
const updates = ref(savedSettings.checkUpdates ?? true)
const checking = ref(false)
const saving = ref(false)
const gpu = computed(() => desktopStore.system?.gpu)

onMounted(async () => {
  if (!isDesktopRuntime()) return
  try {
    const settings = await loadDesktopSettings()
    outputPath.value = settings.outputPath
    tempPath.value = settings.tempPath
    osuPath.value = settings.osuPath
    device.value = settings.device
    precision.value = settings.precision
    workers.value = settings.workers
    autoImport.value = settings.autoImport
    keepIntermediate.value = settings.keepIntermediate
    updates.value = settings.checkUpdates
  } catch (reason) {
    message.error(`Could not load desktop settings: ${String(reason)}`)
  }
})

async function runDiagnostics() {
  checking.value = true
  await desktopStore.initialize()
  checking.value = false
  if (desktopStore.error) message.error(desktopStore.error)
  else message.success('System and installation scan completed.')
}

async function browse(target: 'output' | 'temp' | 'osu') {
  if (!isDesktopRuntime()) {
    message.info('Folder selection is available in the desktop app.')
    return
  }
  const current =
    target === 'output' ? outputPath.value : target === 'temp' ? tempPath.value : osuPath.value
  const selected = await selectDirectory(current || undefined)
  if (!selected) return
  if (target === 'output') outputPath.value = selected
  else if (target === 'temp') tempPath.value = selected
  else osuPath.value = selected
}

async function saveSettings() {
  const settings = {
    outputPath: outputPath.value,
    tempPath: tempPath.value,
    osuPath: osuPath.value,
    device: device.value,
    precision: precision.value,
    workers: workers.value,
    autoImport: autoImport.value,
    keepIntermediate: keepIntermediate.value,
    checkUpdates: updates.value,
  }
  saving.value = true
  try {
    if (isDesktopRuntime()) await saveDesktopSettings(settings)
    else localStorage.setItem('beatforge:settings', JSON.stringify(settings))
    message.success('Settings saved locally.')
  } catch (reason) {
    message.error(`Could not save settings: ${String(reason)}`)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="settings-grid">
    <section class="settings-main">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">Output & storage</h2>
            <p class="section-copy">Choose where beatmaps and temporary files are written.</p>
          </div>
        </div>
        <NForm label-placement="top">
          <NFormItem label="Beatmap output folder">
            <div class="path-field">
              <NInput v-model:value="outputPath" class="mono" /><NButton
                secondary
                @click="browse('output')"
                ><template #icon><NIcon :component="FolderOpenOutline" /></template>Browse</NButton
              >
            </div>
          </NFormItem>
          <NFormItem label="Temporary files folder">
            <div class="path-field">
              <NInput v-model:value="tempPath" class="mono" /><NButton
                secondary
                @click="browse('temp')"
                ><template #icon><NIcon :component="FolderOpenOutline" /></template>Browse</NButton
              >
            </div>
          </NFormItem>
          <NFormItem label="osu! installation folder">
            <div class="path-field">
              <NInput
                v-model:value="osuPath"
                class="mono"
                placeholder="Automatically detected or choose manually"
              /><NButton secondary @click="browse('osu')"
                ><template #icon><NIcon :component="FolderOpenOutline" /></template>Browse</NButton
              >
            </div>
          </NFormItem>
          <div v-if="desktopStore.osuInstallations.length" class="detected-paths">
            <button
              v-for="installation in desktopStore.osuInstallations"
              :key="installation.path"
              type="button"
              @click="osuPath = installation.path"
            >
              <strong>osu! {{ installation.kind }}</strong
              ><span>{{ installation.path }}</span>
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>Import into osu! automatically</strong
              ><small>Open the .osz package when generation finishes.</small>
            </div>
            <NSwitch v-model:value="autoImport" />
          </div>
          <div class="setting-row">
            <div>
              <strong>Keep intermediate files</strong
              ><small>Preserve timing and inference data for debugging.</small>
            </div>
            <NSwitch v-model:value="keepIntermediate" />
          </div>
        </NForm>
      </NCard>

      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">Compute</h2>
            <p class="section-copy">Mapperatorinator inference options for this machine.</p>
          </div>
          <NTag :type="gpu?.available ? 'success' : 'warning'" round size="small">{{
            gpu?.available ? 'CUDA available' : 'CUDA unavailable'
          }}</NTag>
        </div>
        <NForm label-placement="top">
          <NFormItem label="Compute device"
            ><NRadioGroup v-model:value="device"
              ><NRadioButton value="cuda">NVIDIA CUDA</NRadioButton
              ><NRadioButton value="cpu">CPU</NRadioButton></NRadioGroup
            ></NFormItem
          >
          <div class="two-column">
            <NFormItem label="Precision"
              ><NSelect
                v-model:value="precision"
                :options="[
                  { label: 'FP16 · Faster', value: 'fp16' },
                  { label: 'FP32 · Compatible', value: 'fp32' },
                ]"
            /></NFormItem>
            <NFormItem label="Background workers"
              ><NInputNumber v-model:value="workers" :min="1" :max="4"
            /></NFormItem>
          </div>
          <NAlert type="info" :show-icon="false"
            >FP16 is recommended for your RTX 4070 and uses less VRAM.</NAlert
          >
        </NForm>
      </NCard>
    </section>

    <aside class="settings-side">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">Environment</h2>
            <p class="section-copy">Local backend readiness.</p>
          </div>
        </div>
        <div class="health-list">
          <div>
            <span><NIcon :component="CheckmarkCircleOutline" />Desktop core</span
            ><strong>{{ desktopStore.desktop ? 'Connected' : 'Browser mode' }}</strong>
          </div>
          <div>
            <span><NIcon :component="CheckmarkCircleOutline" />Operating system</span
            ><strong>{{
              desktopStore.system
                ? `${desktopStore.system.osName} ${desktopStore.system.osVersion}`
                : 'Checking…'
            }}</strong>
          </div>
          <div>
            <span><NIcon :component="CheckmarkCircleOutline" />Memory</span
            ><strong>{{
              desktopStore.system
                ? `${Math.round(desktopStore.system.totalMemoryMb / 1024)} GB`
                : 'Checking…'
            }}</strong>
          </div>
          <div>
            <span><NIcon :component="HardwareChipOutline" />GPU</span
            ><strong>{{ gpu?.name ?? 'Checking…' }}</strong>
          </div>
          <div>
            <span><NIcon :component="HardwareChipOutline" />Storage free</span
            ><strong>{{
              desktopStore.system ? `${desktopStore.system.availableDiskGb} GB` : 'Checking…'
            }}</strong>
          </div>
        </div>
        <NButton block secondary :loading="checking" @click="runDiagnostics"
          ><template #icon><NIcon :component="RefreshOutline" /></template>Run diagnostics</NButton
        >
      </NCard>
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">Application</h2>
            <p class="section-copy">Update and maintenance preferences.</p>
          </div>
        </div>
        <div class="setting-row">
          <div>
            <strong>Check for updates</strong><small>Look for new app and model releases.</small>
          </div>
          <NSwitch v-model:value="updates" />
        </div>
        <div class="app-version"><span>Beatforge Desktop</span><strong>0.1.0-alpha</strong></div>
      </NCard>
      <NButton type="primary" size="large" block :loading="saving" @click="saveSettings"
        ><template #icon><NIcon :component="SaveOutline" /></template>Save settings</NButton
      >
    </aside>
  </div>
</template>

<style scoped>
.settings-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.45fr) minmax(300px, 0.65fr);
  gap: 20px;
  align-items: start;
}
.settings-main,
.settings-side {
  display: grid;
  gap: 20px;
}
.path-field {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 9px;
  width: 100%;
}
.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  padding: 14px 0;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.setting-row strong,
.setting-row small {
  display: block;
}
.setting-row strong {
  font-size: 12px;
}
.setting-row small {
  margin-top: 3px;
  color: #858da0;
  font-size: 10px;
}
.two-column {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}
.health-list {
  margin-bottom: 18px;
}
.health-list > div {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 11px 0;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
  font-size: 10px;
}
.health-list > div:first-child {
  border-top: 0;
}
.health-list span {
  display: flex;
  align-items: center;
  gap: 7px;
  color: #858da0;
}
.health-list span .n-icon {
  color: #51c790;
}
.health-list strong {
  font-weight: 650;
}
.app-version {
  display: flex;
  justify-content: space-between;
  color: #858da0;
  font-size: 10px;
}
.app-version strong {
  color: inherit;
}
.detected-paths {
  display: grid;
  gap: 6px;
  margin: -4px 0 14px;
}
.detected-paths button {
  padding: 9px 11px;
  text-align: left;
  border: 1px solid rgba(127, 127, 127, 0.14);
  border-radius: 8px;
  color: inherit;
  background: rgba(127, 127, 127, 0.035);
  cursor: pointer;
}
.detected-paths strong,
.detected-paths span {
  display: block;
}
.detected-paths strong {
  font-size: 10px;
}
.detected-paths span {
  margin-top: 2px;
  color: #858da0;
  font-size: 9px;
}
</style>
