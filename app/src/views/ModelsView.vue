<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { NAlert, NButton, NCard, NIcon, NProgress, NSelect, NTag, useMessage } from 'naive-ui'
import {
  CheckmarkCircleOutline,
  CloudDownloadOutline,
  HardwareChipOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import type { ComputeBackend, ManagedPackage } from '@/types'
import { usePackagesStore } from '@/stores/packages'
import { useEnvironmentStore } from '@/stores/environment'
import { useI18n } from '@/i18n'

const message = useMessage()
const packageStore = usePackagesStore()
const environmentStore = useEnvironmentStore()
const { t } = useI18n()
const selectedBackend = ref<ComputeBackend>('cpu')
const runtime = computed(() => packageStore.runtimes[0])
const runtimeBusy = computed(() =>
  ['queued', 'downloading', 'verifying', 'installing', 'removing'].includes(
    runtime.value?.status ?? '',
  ),
)
const models = computed(() => packageStore.models)
const installedModelBytes = computed(() =>
  models.value
    .filter((model) => model.status === 'installed')
    .reduce((total, model) => total + model.sizeBytes, 0),
)
const totalModelBytes = computed(() =>
  models.value.reduce((total, model) => total + model.sizeBytes, 0),
)
const modelStoragePercent = computed(() =>
  totalModelBytes.value ? Math.round((installedModelBytes.value / totalModelBytes.value) * 100) : 0,
)

function runtimeStatusLabel(status?: string) {
  return (
    {
      not_installed: 'Not installed',
      queued: 'Queued',
      downloading: 'Downloading',
      verifying: 'Verifying',
      downloaded: 'Downloaded',
      installing: 'Installing',
      installed: 'Ready',
      update_available: 'Update available',
      removing: 'Removing',
      failed: 'Needs attention',
    }[status ?? ''] ?? 'Checking'
  )
}

function formatBytes(value: number) {
  if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(2)} GB`
  return `${(value / 1024 ** 2).toFixed(0)} MB`
}

function modelBusy(model: ManagedPackage) {
  return ['queued', 'downloading', 'verifying', 'installing', 'removing'].includes(model.status)
}

async function installModel(model: ManagedPackage) {
  try {
    await packageStore.installModel(model.id)
    message.success(`${model.name} is ready.`)
  } catch {
    message.error(packageStore.error || `${model.name} installation failed.`)
  }
}

async function verifyModel(model: ManagedPackage) {
  try {
    await packageStore.verifyModel(model.id)
    message.success(`${model.name} passed checksum verification.`)
  } catch {
    message.error(packageStore.error || `${model.name} verification failed.`)
  }
}

async function removeModel(model: ManagedPackage) {
  try {
    await packageStore.removeModel(model.id)
    message.success(`${model.name} removed.`)
  } catch {
    message.error(packageStore.error || `${model.name} removal failed.`)
  }
}

const backendOptions = computed(() => {
  const cudaDisabled = environmentStore.diagnostics?.compatibility.canGenerateWithCuda === false
  return [
    { label: 'CUDA 13.0', value: 'cuda130', disabled: cudaDisabled },
    { label: 'CUDA 12.8', value: 'cuda128', disabled: cudaDisabled },
    { label: 'CUDA 12.6', value: 'cuda126', disabled: cudaDisabled },
    { label: 'CPU only', value: 'cpu' },
  ] satisfies Array<{ label: string; value: ComputeBackend; disabled?: boolean }>
})

const compatibilityTagType = computed(() => {
  switch (environmentStore.diagnostics?.compatibility.tier) {
    case 'recommended':
      return 'success'
    case 'minimum':
      return 'warning'
    case 'unsupported':
      return 'error'
    default:
      return 'default'
  }
})

const compatibilityTierLabel = computed(() => {
  const tier = environmentStore.diagnostics?.compatibility.tier
  return tier ? t(`models.gpuTier.${tier}`) : t('common.checking')
})

const diagnosticItems = computed(() => {
  const data = environmentStore.diagnostics
  if (!data) return []
  const items = [
    { name: 'Python', ...data.python },
    { name: 'pip', ...data.pip },
    { name: 'FFmpeg', ...data.ffmpeg },
    { name: 'PyTorch', ...data.pytorch },
    { name: 'Mapperatorinator', ...data.mapperatorinator },
    { name: 'Inference lock', ...data.inferenceDependencies },
  ]
  if (environmentStore.workerHealth) {
    items.push({
      name: 'Production worker',
      installed: environmentStore.workerHealth.ready,
      version: `protocol ${environmentStore.workerHealth.protocolVersion} · worker ${environmentStore.workerHealth.workerVersion}`,
      path: environmentStore.workerHealth.pythonExecutable,
      error: environmentStore.workerHealth.errors.join('; ') || undefined,
    })
  }
  return items
})

async function installDependencies() {
  try {
    await environmentStore.install(selectedBackend.value)
    message.success('The Mapperatorinator environment is ready.')
  } catch {
    message.error(environmentStore.error || 'Dependency installation failed.')
  }
}

async function installRuntime() {
  try {
    await packageStore.installRuntime()
    message.success('Private Python runtime is ready.')
  } catch {
    message.error(packageStore.error || 'Python runtime installation failed.')
  }
}

async function checkWorker() {
  try {
    await environmentStore.checkWorker()
    if (environmentStore.workerHealth?.ready) {
      message.success('Production worker health check passed.')
    } else {
      message.warning('Production worker reported missing components.')
    }
  } catch {
    message.error(environmentStore.error || 'Production worker health check failed.')
  }
}

async function verifyRuntime() {
  try {
    const info = await packageStore.verifyRuntime()
    message.success(`Python ${info.version} runtime verified.`)
  } catch {
    message.error(packageStore.error || 'Python runtime verification failed.')
  }
}

async function removeRuntime() {
  try {
    await packageStore.removeRuntime()
    message.success('Private Python runtime removed.')
  } catch {
    message.error(packageStore.error || 'Python runtime removal failed.')
  }
}

watch(
  () => environmentStore.diagnostics?.recommendedBackend,
  (backend) => {
    if (backend) selectedBackend.value = backend
  },
)

onMounted(async () => {
  await packageStore.initialize()
  await environmentStore.initialize()
})
</script>

<template>
  <NCard class="runtime-card panel-card" :bordered="false">
    <div class="runtime-header">
      <div class="model-symbol"><NIcon :component="HardwareChipOutline" /></div>
      <div class="runtime-copy">
        <div class="model-name">
          <h2>{{ runtime?.name ?? t('models.runtime') }}</h2>
          <NTag
            size="small"
            round
            :type="runtime?.status === 'installed' ? 'success' : runtime?.status === 'failed' ? 'error' : 'default'"
          >
            {{ runtimeStatusLabel(runtime?.status) }}
          </NTag>
          <NTag v-if="runtime" size="small" round>v{{ runtime.targetVersion }}</NTag>
        </div>
        <p>
          {{ t('models.runtimeHelp') }}
        </p>
        <span v-if="runtime?.installPath" class="runtime-path">{{ runtime.installPath }}</span>
        <NProgress
          v-if="runtimeBusy && runtime"
          type="line"
          :percentage="runtime.progress"
          :height="6"
          color="#ff66aa"
        />
      </div>
      <div class="runtime-actions">
        <NButton
          v-if="runtime?.status === 'installed'"
          secondary
          :disabled="runtimeBusy"
          @click="verifyRuntime"
        >
          {{ t('common.verify') }}
        </NButton>
        <NButton
          v-if="runtime && runtime.status !== 'installed'"
          type="primary"
          secondary
          :loading="runtimeBusy"
          :disabled="runtimeBusy"
          @click="installRuntime"
        >
          {{ t('models.installRuntime') }}
        </NButton>
        <NButton
          v-if="runtime && ['installed', 'downloaded', 'failed'].includes(runtime.status)"
          quaternary
          type="error"
          :disabled="runtimeBusy"
          @click="removeRuntime"
        >
          {{ t('common.remove') }}
        </NButton>
      </div>
    </div>
    <NAlert v-if="packageStore.error" type="error" class="runtime-error">
      {{ packageStore.error }}
    </NAlert>
  </NCard>
  <NCard class="environment-card panel-card" :bordered="false">
    <div class="environment-header">
      <div>
        <div class="model-name">
          <h2>{{ t('models.environment') }}</h2>
          <NTag
            size="small"
            round
            :type="environmentStore.diagnostics?.ready ? 'success' : 'warning'"
          >
            {{ environmentStore.diagnostics?.ready ? t('common.ready') : t('models.setupRequired') }}
          </NTag>
        </div>
        <p class="environment-summary">
          {{ environmentStore.diagnostics?.compatibilityReason ?? t('models.compatibility') }}
        </p>
        <span v-if="environmentStore.diagnostics?.gpuName" class="runtime-path">
          {{ environmentStore.diagnostics.gpuName }} · Driver {{ environmentStore.diagnostics.driverVersion }}
        </span>
      </div>
      <div class="environment-actions">
        <NSelect
          v-model:value="selectedBackend"
          :options="backendOptions"
          :disabled="environmentStore.installing"
        />
        <NButton
          type="primary"
          secondary
          :loading="environmentStore.installing"
          :disabled="runtime?.status !== 'installed' || environmentStore.installing"
          @click="installDependencies"
        >
          {{ environmentStore.diagnostics?.ready ? t('models.repairEnvironment') : t('models.installEnvironment') }}
        </NButton>
        <NButton
          secondary
          :loading="environmentStore.loading"
          :disabled="environmentStore.installing"
          @click="environmentStore.refresh"
        >
          {{ t('models.diagnose') }}
        </NButton>
        <NButton
          secondary
          :loading="environmentStore.healthChecking"
          :disabled="runtime?.status !== 'installed' || environmentStore.installing"
          @click="checkWorker"
        >
          {{ t('models.workerHealth') }}
        </NButton>
      </div>
    </div>
    <div v-if="environmentStore.diagnostics" class="compatibility-panel">
      <div class="compatibility-title">
        <strong>{{ t('models.gpuCompatibility') }}</strong>
        <NTag size="small" round :type="compatibilityTagType">{{ compatibilityTierLabel }}</NTag>
      </div>
      <div class="compatibility-facts">
        <span>{{ t('models.gpuArchitecture') }} <strong>{{ environmentStore.diagnostics.compatibility.architecture }}</strong></span>
        <span>{{ t('models.gpuVram') }} <strong>{{ environmentStore.diagnostics.gpuMemoryTotalMb ? `${environmentStore.diagnostics.gpuMemoryTotalMb} MiB` : '—' }}</strong></span>
        <span>{{ t('models.gpuFreeVram') }} <strong>{{ environmentStore.diagnostics.gpuMemoryFreeMb ? `${environmentStore.diagnostics.gpuMemoryFreeMb} MiB` : '—' }}</strong></span>
        <span>BF16 <strong>{{ environmentStore.diagnostics.compatibility.bf16Supported ? t('models.nativeSupported') : t('models.fp32Fallback') }}</strong></span>
      </div>
      <NAlert
        v-if="environmentStore.diagnostics.compatibility.recommendations.length"
        :type="environmentStore.diagnostics.compatibility.canGenerateWithCuda ? 'info' : 'warning'"
        class="compatibility-advice"
      >
        {{ environmentStore.diagnostics.compatibility.recommendations.join(' ') }}
      </NAlert>
    </div>
    <div class="diagnostic-grid">
      <div v-for="item in diagnosticItems" :key="item.name" class="diagnostic-item">
        <span>{{ item.name }}</span>
        <NTag size="small" round :type="item.installed ? 'success' : 'default'">
          {{ item.installed ? item.version ?? t('models.installed') : t('models.missing') }}
        </NTag>
      </div>
    </div>
    <div v-if="environmentStore.installing && environmentStore.installEvent" class="environment-progress">
      <NProgress
        type="line"
        :percentage="environmentStore.installEvent.progress"
        :height="6"
        color="#ff66aa"
      />
      <span>{{ environmentStore.installEvent.message }}</span>
    </div>
    <NAlert v-if="environmentStore.error" type="error" class="runtime-error">
      {{ environmentStore.error }}
    </NAlert>
    <NAlert
      v-if="environmentStore.workerHealth && !environmentStore.workerHealth.ready"
      type="warning"
      class="runtime-error"
    >
      {{ environmentStore.workerHealth.errors.join(' · ') }}
    </NAlert>
  </NCard>
  <NCard class="storage-banner panel-card" :bordered="false">
    <div class="storage-icon"><NIcon :component="HardwareChipOutline" /></div>
    <div class="storage-copy">
      <strong>{{ t('models.storage') }}</strong><span>{{ t('models.storageHelp') }}</span
      ><NProgress
        type="line"
        :percentage="modelStoragePercent"
        :show-indicator="false"
        :height="6"
        color="#ff66aa"
      />
    </div>
    <div class="storage-value">
      <strong>{{ formatBytes(installedModelBytes) }}</strong
      ><span>{{ t('models.storageValue', { total: formatBytes(totalModelBytes) }) }}</span>
    </div>
  </NCard>
  <div class="model-list">
    <NCard v-for="model in models" :key="model.id" class="panel-card model-card" :bordered="false">
      <div class="model-row">
        <div class="model-symbol"><NIcon :component="HardwareChipOutline" /></div>
        <div class="model-info">
          <div class="model-name">
            <h2>{{ model.name }}</h2>
            <NTag v-if="model.recommended" size="small" type="success" round>{{ t('models.recommended') }}</NTag
            ><NTag size="small" round>v{{ model.targetVersion }}</NTag>
          </div>
          <p>{{ model.description }}</p>
          <div class="model-meta">
            <span>{{ formatBytes(model.sizeBytes) }}</span
            ><span>osu!standard</span><span>{{ t('models.official') }}</span>
          </div>
          <div v-if="modelBusy(model)" class="download-progress">
            <NProgress type="line" :percentage="model.progress" :height="6" color="#ff66aa" /><span
              >{{ runtimeStatusLabel(model.status) }} model package…</span
            >
          </div>
        </div>
        <div class="model-action">
          <NTag v-if="model.status === 'installed'" type="success" :bordered="false"
            ><template #icon><NIcon :component="CheckmarkCircleOutline" /></template>{{ t('models.installed') }}</NTag
          >
          <NButton
            v-if="model.status !== 'installed'"
            type="primary"
            secondary
            :loading="modelBusy(model)"
            :disabled="modelBusy(model) || runtime?.status !== 'installed'"
            @click="installModel(model)"
            ><template #icon><NIcon :component="CloudDownloadOutline" /></template>
            {{ model.status === 'update_available' ? t('models.update') : model.status === 'failed' ? t('models.repair') : t('common.install') }}
          </NButton
          >
          <NButton
            v-if="model.status === 'installed'"
            secondary
            :disabled="modelBusy(model)"
            @click="verifyModel(model)"
          >{{ t('common.verify') }}</NButton>
          <NButton
            v-if="['installed', 'downloaded', 'failed', 'update_available'].includes(model.status)"
            quaternary
            circle
            :aria-label="t('common.remove')"
            :disabled="modelBusy(model)"
            @click="removeModel(model)"
            ><template #icon><NIcon :component="TrashOutline" /></template
          ></NButton>
        </div>
      </div>
    </NCard>
  </div>
</template>

<style scoped>
.runtime-card {
  margin-bottom: 14px;
}
.environment-card {
  margin-bottom: 14px;
}
.environment-header {
  display: grid;
  grid-template-columns: 1fr minmax(420px, auto);
  gap: 20px;
  align-items: start;
}
.environment-summary {
  margin: 7px 0;
  color: #858da0;
  font-size: 11px;
}
.environment-actions {
  display: grid;
  grid-template-columns: 140px auto auto auto;
  gap: 8px;
}
.diagnostic-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
  margin-top: 16px;
}
.compatibility-panel {
  margin-top: 16px;
  padding: 12px;
  border: 1px solid rgba(133, 141, 160, 0.16);
  border-radius: 10px;
}
.compatibility-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 11px;
}
.compatibility-facts {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  margin-top: 10px;
  color: #858da0;
  font-size: 9px;
}
.compatibility-facts span {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.compatibility-facts strong {
  color: var(--text-color-2);
  font-size: 10px;
}
.compatibility-advice {
  margin-top: 10px;
}
.diagnostic-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid rgba(133, 141, 160, 0.16);
  border-radius: 10px;
  font-size: 10px;
}
.environment-progress {
  margin-top: 14px;
}
.environment-progress span {
  display: block;
  margin-top: 5px;
  color: #858da0;
  font-size: 9px;
}
.runtime-header {
  display: grid;
  grid-template-columns: 54px 1fr auto;
  gap: 17px;
  align-items: start;
}
.runtime-copy p {
  margin: 7px 0;
  color: #858da0;
  font-size: 11px;
}
.runtime-path {
  display: block;
  margin-bottom: 10px;
  color: #858da0;
  font-family: monospace;
  font-size: 9px;
  overflow-wrap: anywhere;
}
.runtime-actions {
  display: flex;
  gap: 8px;
}
.runtime-error {
  margin-top: 14px;
}
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
