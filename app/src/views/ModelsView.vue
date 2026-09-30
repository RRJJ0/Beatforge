<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NCard, NIcon, NProgress, NTag, useMessage } from 'naive-ui'
import {
  CheckmarkCircleOutline,
  CloudDownloadOutline,
  HardwareChipOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import type { ModelPackage } from '@/types'
import { simulateProgress } from '@/services/mockBackend'

const message = useMessage()
const models = ref<ModelPackage[]>([
  {
    id: 'standard',
    name: 'Mapperatorinator Standard',
    version: '1.3.0',
    description: 'Balanced model for most songs and 2–6★ beatmaps.',
    size: '2.8 GB',
    status: 'installed',
    recommended: true,
  },
  {
    id: 'high-sr',
    name: 'Mapperatorinator High SR',
    version: '1.1.2',
    description: 'Denser patterns designed for advanced 5–8★ maps.',
    size: '3.4 GB',
    status: 'available',
  },
  {
    id: 'legacy',
    name: 'Mapperatorinator Legacy',
    version: '0.9.7',
    description: 'Compatibility model for older generation presets.',
    size: '2.1 GB',
    status: 'installed',
  },
])

function download(model: ModelPackage) {
  model.status = 'downloading'
  model.progress = 0
  message.info(`Started downloading ${model.name}.`)
  simulateProgress(
    (progress) => {
      model.progress = progress
    },
    () => {
      model.status = 'installed'
      model.progress = undefined
      message.success(`${model.name} is ready to use.`)
    },
  )
}
function remove(model: ModelPackage) {
  model.status = 'available'
  model.progress = undefined
  message.success(`${model.name} removed from local storage.`)
}
</script>

<template>
  <NCard class="storage-banner panel-card" :bordered="false">
    <div class="storage-icon"><NIcon :component="HardwareChipOutline" /></div>
    <div class="storage-copy">
      <strong>Model storage</strong><span>C:\Users\You\.beatforge\models</span
      ><NProgress
        type="line"
        :percentage="47"
        :show-indicator="false"
        :height="6"
        color="#ff66aa"
      />
    </div>
    <div class="storage-value"><strong>8.9 GB</strong><span>of 19 GB used</span></div>
  </NCard>
  <div class="model-list">
    <NCard v-for="model in models" :key="model.id" class="panel-card model-card" :bordered="false">
      <div class="model-row">
        <div class="model-symbol"><NIcon :component="HardwareChipOutline" /></div>
        <div class="model-info">
          <div class="model-name">
            <h2>{{ model.name }}</h2>
            <NTag v-if="model.recommended" size="small" type="success" round>Recommended</NTag
            ><NTag size="small" round>v{{ model.version }}</NTag>
          </div>
          <p>{{ model.description }}</p>
          <div class="model-meta">
            <span>{{ model.size }}</span
            ><span>osu!standard</span><span>Updated Sep 2026</span>
          </div>
          <div v-if="model.status === 'downloading'" class="download-progress">
            <NProgress type="line" :percentage="model.progress" :height="6" color="#ff66aa" /><span
              >Downloading model package…</span
            >
          </div>
        </div>
        <div class="model-action">
          <NTag v-if="model.status === 'installed'" type="success" :bordered="false"
            ><template #icon><NIcon :component="CheckmarkCircleOutline" /></template>Installed</NTag
          >
          <NButton
            v-if="model.status === 'available'"
            type="primary"
            secondary
            @click="download(model)"
            ><template #icon><NIcon :component="CloudDownloadOutline" /></template>Download</NButton
          >
          <NButton
            v-if="model.status === 'installed' && !model.recommended"
            quaternary
            circle
            @click="remove(model)"
            ><template #icon><NIcon :component="TrashOutline" /></template
          ></NButton>
        </div>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.storage-banner {
  margin-bottom: 20px;
}
.storage-banner :deep(.n-card__content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
.storage-icon,
.model-symbol {
  display: grid;
  place-items: center;
  color: #ff66aa;
  background: rgba(255, 102, 170, 0.12);
}
.storage-icon {
  width: 46px;
  height: 46px;
  border-radius: 12px;
  font-size: 22px;
}
.storage-copy {
  flex: 1;
}
.storage-copy strong,
.storage-copy span {
  display: block;
}
.storage-copy strong {
  font-size: 13px;
}
.storage-copy span {
  margin: 3px 0 10px;
  color: #858da0;
  font-size: 10px;
}
.storage-value {
  text-align: right;
}
.storage-value strong,
.storage-value span {
  display: block;
}
.storage-value strong {
  font-size: 16px;
}
.storage-value span {
  color: #858da0;
  font-size: 10px;
}
.model-list {
  display: grid;
  gap: 14px;
}
.model-row {
  display: grid;
  grid-template-columns: 54px 1fr auto;
  gap: 17px;
  align-items: start;
}
.model-symbol {
  width: 54px;
  height: 54px;
  border-radius: 13px;
  font-size: 24px;
}
.model-name {
  display: flex;
  align-items: center;
  gap: 8px;
}
.model-name h2 {
  margin: 0;
  font-size: 14px;
}
.model-info p {
  margin: 7px 0;
  color: #858da0;
  font-size: 11px;
}
.model-meta {
  display: flex;
  gap: 15px;
  color: #858da0;
  font-size: 9px;
}
.model-action {
  display: flex;
  align-items: center;
  gap: 5px;
}
.download-progress {
  max-width: 420px;
  margin-top: 14px;
}
.download-progress span {
  display: block;
  margin-top: 5px;
  color: #858da0;
  font-size: 9px;
}
</style>
