import { ref } from 'vue'
import { defineStore } from 'pinia'
import {
  diagnoseAiEnvironment,
  diagnoseProductionWorker,
  installAiDependencies,
  isDesktopRuntime,
  onEnvironmentInstallEvent,
} from '@/services/desktop'
import type {
  ComputeBackend,
  EnvironmentDiagnostics,
  EnvironmentInstallEvent,
  WorkerHealth,
} from '@/types'

export const useEnvironmentStore = defineStore('environment', () => {
  const diagnostics = ref<EnvironmentDiagnostics>()
  const loading = ref(false)
  const installing = ref(false)
  const error = ref('')
  const installEvent = ref<EnvironmentInstallEvent>()
  const workerHealth = ref<WorkerHealth>()
  const healthChecking = ref(false)
  let listening = false

  async function initialize() {
    if (!isDesktopRuntime()) return
    if (!listening) {
      listening = true
      await onEnvironmentInstallEvent((event) => {
        installEvent.value = event
      })
    }
    await refresh()
  }

  async function refresh() {
    if (!isDesktopRuntime() || loading.value || installing.value) return
    loading.value = true
    error.value = ''
    try {
      diagnostics.value = await diagnoseAiEnvironment()
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
    } finally {
      loading.value = false
    }
  }

  async function install(backend: ComputeBackend) {
    if (!isDesktopRuntime() || installing.value) return
    installing.value = true
    error.value = ''
    installEvent.value = {
      stage: 'preparing',
      progress: 0,
      message: 'Preparing Mapperatorinator environment installation',
    }
    try {
      diagnostics.value = await installAiDependencies(backend)
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      throw reason
    } finally {
      installing.value = false
    }
  }

  async function checkWorker() {
    if (!isDesktopRuntime() || healthChecking.value || installing.value) return
    healthChecking.value = true
    error.value = ''
    try {
      workerHealth.value = await diagnoseProductionWorker()
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      throw reason
    } finally {
      healthChecking.value = false
    }
  }

  return {
    diagnostics,
    workerHealth,
    loading,
    installing,
    healthChecking,
    error,
    installEvent,
    initialize,
    refresh,
    install,
    checkWorker,
  }
})
