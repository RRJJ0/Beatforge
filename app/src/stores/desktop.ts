import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  detectOsuInstallations,
  getAppPaths,
  getSystemInfo,
  isDesktopRuntime,
} from '@/services/desktop'
import type { AppPaths, OsuInstallation, SystemInfo } from '@/types'

export const useDesktopStore = defineStore('desktop', () => {
  const paths = ref<AppPaths | null>(null)
  const system = ref<SystemInfo | null>(null)
  const osuInstallations = ref<OsuInstallation[]>([])
  const loading = ref(false)
  const error = ref('')
  const desktop = isDesktopRuntime()

  const engineLabel = computed(() => {
    if (!desktop) return 'Browser prototype'
    if (!system.value) return 'Checking hardware…'
    return system.value.gpu.available
      ? `${system.value.gpu.name} · Ready`
      : 'CPU mode · GPU unavailable'
  })
  const engineDetail = computed(() => {
    const gpu = system.value?.gpu
    if (!gpu?.available) return desktop ? 'NVIDIA CUDA not detected' : 'Run with pnpm tauri dev'
    return `${gpu.driverVersion ?? 'Driver unknown'} · ${gpu.memoryFreeMb ?? 0} MB free`
  })

  async function initialize() {
    if (!desktop || loading.value) return
    loading.value = true
    error.value = ''
    try {
      const [appPaths, systemInfo, installations] = await Promise.all([
        getAppPaths(),
        getSystemInfo(),
        detectOsuInstallations(),
      ])
      paths.value = appPaths
      system.value = systemInfo
      osuInstallations.value = installations
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
    } finally {
      loading.value = false
    }
  }

  return {
    paths,
    system,
    osuInstallations,
    loading,
    error,
    desktop,
    engineLabel,
    engineDetail,
    initialize,
  }
})
