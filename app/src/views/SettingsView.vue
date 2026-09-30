<script setup lang="ts">
import { ref } from 'vue'
import {
  NAlert, NButton, NCard, NForm, NFormItem, NIcon, NInput, NInputNumber,
  NRadioButton, NRadioGroup, NSelect, NSwitch, NTag, useMessage,
} from 'naive-ui'
import { CheckmarkCircleOutline, FolderOpenOutline, HardwareChipOutline, RefreshOutline, SaveOutline } from '@vicons/ionicons5'
import { mockEnvironmentCheck } from '@/services/mockBackend'

const message = useMessage()
const savedSettings = JSON.parse(localStorage.getItem('beatforge:settings') ?? '{}')
const outputPath = ref(savedSettings.outputPath ?? 'C:\\Users\\You\\Documents\\osu!\\Songs\\Beatforge')
const tempPath = ref(savedSettings.tempPath ?? 'C:\\Users\\You\\AppData\\Local\\Beatforge\\cache')
const device = ref(savedSettings.device ?? 'cuda')
const precision = ref(savedSettings.precision ?? 'fp16')
const workers = ref(savedSettings.workers ?? 2)
const autoImport = ref(savedSettings.autoImport ?? true)
const keepIntermediate = ref(savedSettings.keepIntermediate ?? false)
const updates = ref(savedSettings.updates ?? true)
const checking = ref(false)

async function runDiagnostics() {
  checking.value = true
  await mockEnvironmentCheck()
  checking.value = false
  message.success('Environment check completed. All components are ready.')
}

function saveSettings() {
  localStorage.setItem('beatforge:settings', JSON.stringify({ outputPath: outputPath.value, tempPath: tempPath.value, device: device.value, precision: precision.value, workers: workers.value, autoImport: autoImport.value, keepIntermediate: keepIntermediate.value, updates: updates.value }))
  message.success('Settings saved locally.')
}
</script>

<template>
  <div class="settings-grid">
    <section class="settings-main">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading"><div><h2 class="section-title">Output & storage</h2><p class="section-copy">Choose where beatmaps and temporary files are written.</p></div></div>
        <NForm label-placement="top">
          <NFormItem label="Beatmap output folder">
            <div class="path-field"><NInput v-model:value="outputPath" class="mono" /><NButton secondary><template #icon><NIcon :component="FolderOpenOutline" /></template>Browse</NButton></div>
          </NFormItem>
          <NFormItem label="Temporary files folder">
            <div class="path-field"><NInput v-model:value="tempPath" class="mono" /><NButton secondary><template #icon><NIcon :component="FolderOpenOutline" /></template>Browse</NButton></div>
          </NFormItem>
          <div class="setting-row"><div><strong>Import into osu! automatically</strong><small>Open the .osz package when generation finishes.</small></div><NSwitch v-model:value="autoImport" /></div>
          <div class="setting-row"><div><strong>Keep intermediate files</strong><small>Preserve timing and inference data for debugging.</small></div><NSwitch v-model:value="keepIntermediate" /></div>
        </NForm>
      </NCard>

      <NCard class="panel-card" :bordered="false">
        <div class="card-heading"><div><h2 class="section-title">Compute</h2><p class="section-copy">Mapperatorinator inference options for this machine.</p></div><NTag type="success" round size="small">CUDA available</NTag></div>
        <NForm label-placement="top">
          <NFormItem label="Compute device"><NRadioGroup v-model:value="device"><NRadioButton value="cuda">NVIDIA CUDA</NRadioButton><NRadioButton value="cpu">CPU</NRadioButton></NRadioGroup></NFormItem>
          <div class="two-column">
            <NFormItem label="Precision"><NSelect v-model:value="precision" :options="[{label:'FP16 · Faster',value:'fp16'},{label:'FP32 · Compatible',value:'fp32'}]" /></NFormItem>
            <NFormItem label="Background workers"><NInputNumber v-model:value="workers" :min="1" :max="4" /></NFormItem>
          </div>
          <NAlert type="info" :show-icon="false">FP16 is recommended for your RTX 4070 and uses less VRAM.</NAlert>
        </NForm>
      </NCard>
    </section>

    <aside class="settings-side">
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading"><div><h2 class="section-title">Environment</h2><p class="section-copy">Local backend readiness.</p></div></div>
        <div class="health-list">
          <div><span><NIcon :component="CheckmarkCircleOutline" />Python runtime</span><strong>3.11.9</strong></div>
          <div><span><NIcon :component="CheckmarkCircleOutline" />Mapperatorinator</span><strong>Ready</strong></div>
          <div><span><NIcon :component="CheckmarkCircleOutline" />PyTorch + CUDA</span><strong>2.5 · 12.4</strong></div>
          <div><span><NIcon :component="HardwareChipOutline" />GPU</span><strong>RTX 4070</strong></div>
        </div>
        <NButton block secondary :loading="checking" @click="runDiagnostics"><template #icon><NIcon :component="RefreshOutline" /></template>Run diagnostics</NButton>
      </NCard>
      <NCard class="panel-card" :bordered="false">
        <div class="card-heading"><div><h2 class="section-title">Application</h2><p class="section-copy">Update and maintenance preferences.</p></div></div>
        <div class="setting-row"><div><strong>Check for updates</strong><small>Look for new app and model releases.</small></div><NSwitch v-model:value="updates" /></div>
        <div class="app-version"><span>Beatforge Desktop</span><strong>0.1.0-alpha</strong></div>
      </NCard>
      <NButton type="primary" size="large" block @click="saveSettings"><template #icon><NIcon :component="SaveOutline" /></template>Save settings</NButton>
    </aside>
  </div>
</template>

<style scoped>
.settings-grid { display: grid; grid-template-columns: minmax(0,1.45fr) minmax(300px,.65fr); gap: 20px; align-items: start; }.settings-main,.settings-side { display: grid; gap: 20px; }.path-field { display: grid; grid-template-columns: 1fr auto; gap: 9px; width: 100%; }.setting-row { display: flex; justify-content: space-between; align-items: center; gap: 20px; padding: 14px 0; border-top: 1px solid rgba(127,127,127,.12); }.setting-row strong,.setting-row small { display: block; }.setting-row strong { font-size: 12px; }.setting-row small { margin-top: 3px; color: #858da0; font-size: 10px; }.two-column { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }.health-list { margin-bottom: 18px; }.health-list > div { display: flex; justify-content: space-between; align-items: center; padding: 11px 0; border-top: 1px solid rgba(127,127,127,.12); font-size: 10px; }.health-list > div:first-child { border-top: 0; }.health-list span { display: flex; align-items: center; gap: 7px; color: #858da0; }.health-list span .n-icon { color: #51c790; }.health-list strong { font-weight: 650; }.app-version { display: flex; justify-content: space-between; color: #858da0; font-size: 10px; }.app-version strong { color: inherit; }
</style>

