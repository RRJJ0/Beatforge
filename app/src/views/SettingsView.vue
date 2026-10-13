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
  NModal,
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
  exportDesktopSettings,
  loadDesktopSettings,
  previewDesktopSettingsImport,
  resetDesktopSettings,
  saveDesktopSettings,
  selectDirectory,
  selectSettingsExportFile,
  selectSettingsImportFile,
} from '@/services/desktop'
import { useDesktopStore } from '@/stores/desktop'
import type { AppSettings, SettingsImportPreview } from '@/types'

const message = useMessage()
const desktopStore = useDesktopStore()
const savedSettings = JSON.parse(localStorage.getItem('beatforge:settings') ?? '{}')
const outputPath = ref(savedSettings.outputPath ?? '')
const tempPath = ref(savedSettings.tempPath ?? '')
const osuPath = ref(savedSettings.osuPath ?? '')
const device = ref(savedSettings.device ?? 'cuda')
const precision = ref(
  savedSettings.precision === 'fp16' ? 'bf16' : (savedSettings.precision ?? 'bf16'),
)
const workers = ref(savedSettings.workers ?? 2)
const autoImport = ref(savedSettings.autoImport ?? true)
const keepIntermediate = ref(savedSettings.keepIntermediate ?? false)
const repairGeneratedOutput = ref(savedSettings.repairGeneratedOutput ?? true)
const updates = ref(savedSettings.checkUpdates ?? true)
const checking = ref(false)
const saving = ref(false)
const transferring = ref(false)
const resetField = ref<keyof AppSettings>('outputPath')
const importPreview = ref<SettingsImportPreview | null>(null)
const gpu = computed(() => desktopStore.system?.gpu)

function applySettings(settings: AppSettings) {
  outputPath.value = settings.outputPath
  tempPath.value = settings.tempPath
  osuPath.value = settings.osuPath
  device.value = settings.device
  precision.value = settings.precision === 'fp16' ? 'bf16' : settings.precision
  workers.value = settings.workers
  autoImport.value = settings.autoImport
  keepIntermediate.value = settings.keepIntermediate
  repairGeneratedOutput.value = settings.repairGeneratedOutput
  updates.value = settings.checkUpdates
}

function currentSettings(): AppSettings {
  return {
    outputPath: outputPath.value,
    tempPath: tempPath.value,
    osuPath: osuPath.value,
    device: device.value,
    precision: precision.value,
    workers: workers.value,
    autoImport: autoImport.value,
    keepIntermediate: keepIntermediate.value,
    repairGeneratedOutput: repairGeneratedOutput.value,
    checkUpdates: updates.value,
  }
}

onMounted(async () => {
  if (!isDesktopRuntime()) return
  try {
    applySettings(await loadDesktopSettings())
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
  const settings = currentSettings()
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

async function exportSettingsFile() {
  if (!isDesktopRuntime()) return message.info('Settings export is available in the desktop app.')
  const path = await selectSettingsExportFile()
  if (!path) return
  transferring.value = true
  try {
    const exported = await exportDesktopSettings(path, currentSettings())
    message.success(`Settings exported to ${exported}`)
  } catch (reason) {
    message.error(`Could not export settings: ${String(reason)}`)
  } finally {
    transferring.value = false
  }
}

async function chooseSettingsImport() {
  if (!isDesktopRuntime()) return message.info('Settings import is available in the desktop app.')
  const path = await selectSettingsImportFile()
  if (!path) return
  transferring.value = true
  try {
    importPreview.value = await previewDesktopSettingsImport(path)
  } catch (reason) {
    message.error(`Could not preview settings import: ${String(reason)}`)
  } finally {
    transferring.value = false
  }
}

async function applySettingsImport() {
  if (!importPreview.value) return
  transferring.value = true
  try {
    await saveDesktopSettings(importPreview.value.resultingSettings)
    applySettings(importPreview.value.resultingSettings)
    importPreview.value = null
    message.success('Imported settings applied.')
  } catch (reason) {
    message.error(`Could not apply imported settings: ${String(reason)}`)
  } finally {
    transferring.value = false
  }
}

async function restoreDefaults(all: boolean) {
  if (!isDesktopRuntime()) return message.info('Settings reset is available in the desktop app.')
  if (all && !window.confirm('Restore every setting to this machine’s defaults?')) return
  transferring.value = true
  try {
    const settings = await resetDesktopSettings(all ? undefined : resetField.value)
    applySettings(settings)
    message.success(all ? 'All settings restored.' : `${resetField.value} restored.`)
  } catch (reason) {
    message.error(`Could not restore settings: ${String(reason)}`)
  } finally {
    transferring.value = false
  }
}

function settingValue(value: unknown) {
  if (value === undefined) return 'Not present'
  if (value === '') return 'Empty'
  return typeof value === 'string' ? value : JSON.stringify(value)
}

function importTagType(action: SettingsImportPreview['items'][number]['action']) {
  if (action === 'apply') return 'success'
  if (action === 'ignored') return 'warning'
  if (action === 'reset') return 'info'
  return 'default'
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
              :disabled="!installation.executable"
              :title="
                installation.executable ? 'Use this osu! installation' : 'No osu!.exe detected here'
              "
              @click="osuPath = installation.path"
            >
              <strong>osu! {{ installation.kind }}</strong
              ><span>{{ installation.path }}</span>
            </button>
          </div>
          <div class="setting-row">
            <div>
              <strong>Import into osu! automatically</strong
              ><small>Open validated output after generation finishes.</small>
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
                  { label: 'BF16 · Recommended', value: 'bf16' },
                  { label: 'FP32 · Compatible', value: 'fp32' },
                ]"
            /></NFormItem>
            <NFormItem label="Background workers"
              ><NInputNumber v-model:value="workers" :min="1" :max="4"
            /></NFormItem>
          </div>
          <NAlert type="info" :show-icon="false"
            >BF16 is the native precision for Mapperatorinator V32. Unsupported GPUs fall back
            safely to FP32.</NAlert
          >
          <div class="setting-row">
            <div>
              <strong>Repair unsafe generated gameplay data</strong
              ><small
                >Resolve generated event times, discard time regressions, and repair invalid slider
                values. Disable to preserve Mapperatorinator output unchanged.</small
              >
            </div>
            <NSwitch v-model:value="repairGeneratedOutput" />
          </div>
          <NAlert v-if="!repairGeneratedOutput" type="warning" :show-icon="false">
            Raw output can fail validation or contain unplayable objects. Failed output can still be
            imported manually from the job details.
          </NAlert>
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
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading">
          <div>
            <h2 class="section-title">Backup & reset</h2>
            <p class="section-copy">Versioned JSON settings without model or song data.</p>
          </div>
          <NTag round size="small">Schema v1</NTag>
        </div>
        <div class="transfer-actions">
          <NButton secondary :loading="transferring" @click="exportSettingsFile">
            Export settings
          </NButton>
          <NButton secondary :loading="transferring" @click="chooseSettingsImport">
            Import settings
          </NButton>
        </div>
        <div class="reset-control">
          <NSelect
            v-model:value="resetField"
            :options="[
              { label: 'Output path', value: 'outputPath' },
              { label: 'Temporary path', value: 'tempPath' },
              { label: 'osu! path', value: 'osuPath' },
              { label: 'Compute device', value: 'device' },
              { label: 'Precision', value: 'precision' },
              { label: 'Background workers', value: 'workers' },
              { label: 'Automatic import', value: 'autoImport' },
              { label: 'Keep intermediate files', value: 'keepIntermediate' },
              { label: 'Generated output repair', value: 'repairGeneratedOutput' },
              { label: 'Update checks', value: 'checkUpdates' },
            ]"
          />
          <NButton :loading="transferring" @click="restoreDefaults(false)">Reset selected</NButton>
        </div>
        <NButton
          block
          tertiary
          type="warning"
          :loading="transferring"
          @click="restoreDefaults(true)"
        >
          Restore all defaults
        </NButton>
      </NCard>
      <NButton type="primary" size="large" block :loading="saving" @click="saveSettings"
        ><template #icon><NIcon :component="SaveOutline" /></template>Save settings</NButton
      >
    </aside>
  </div>

  <NModal :show="Boolean(importPreview)" @update:show="(show) => !show && (importPreview = null)">
    <NCard class="import-preview" :bordered="false" role="dialog" aria-modal="true">
      <div class="card-heading">
        <div>
          <h2 class="section-title">Review settings import</h2>
          <p class="section-copy">
            {{
              importPreview?.schemaVersion === 0
                ? 'Legacy settings'
                : `Schema v${importPreview?.schemaVersion}`
            }}
            · <span class="mono">{{ importPreview?.sourcePath }}</span>
          </p>
        </div>
      </div>
      <NAlert type="info" :show-icon="false">
        Machine-specific paths are applied only when they exist. Confirming changes settings only;
        models, jobs, and songs are not imported.
      </NAlert>
      <div class="import-items">
        <div v-for="item in importPreview?.items" :key="item.field" class="import-item">
          <div>
            <strong>{{ item.field }}</strong>
            <small>{{ item.reason }}</small>
          </div>
          <div class="import-change">
            <span class="mono">{{ settingValue(item.current) }}</span>
            <span>→</span>
            <span class="mono">{{ settingValue(item.result) }}</span>
          </div>
          <NTag size="small" round :type="importTagType(item.action)">{{ item.action }}</NTag>
        </div>
      </div>
      <div class="modal-actions">
        <NButton @click="importPreview = null">Cancel</NButton>
        <NButton type="primary" :loading="transferring" @click="applySettingsImport">
          Apply imported settings
        </NButton>
      </div>
    </NCard>
  </NModal>
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
.transfer-actions,
.reset-control {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.reset-control {
  margin: 14px 0 8px;
}
.import-preview {
  width: min(820px, calc(100vw - 48px));
  max-height: calc(100vh - 64px);
  overflow: auto;
}
.import-items {
  display: grid;
  margin-top: 14px;
}
.import-item {
  display: grid;
  grid-template-columns: minmax(180px, 0.8fr) minmax(280px, 1.4fr) 90px;
  gap: 14px;
  align-items: center;
  padding: 11px 0;
  border-top: 1px solid rgba(127, 127, 127, 0.12);
}
.import-item strong,
.import-item small {
  display: block;
}
.import-item small {
  margin-top: 3px;
  color: #858da0;
  font-size: 9px;
}
.import-change {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  gap: 8px;
  align-items: center;
  color: #858da0;
  font-size: 9px;
}
.import-change .mono {
  overflow-wrap: anywhere;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
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
.detected-paths button:disabled {
  cursor: not-allowed;
  opacity: 0.48;
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
