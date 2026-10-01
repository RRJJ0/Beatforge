import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  downloadManagedPackage,
  installPythonRuntime,
  installModelPackage,
  isDesktopRuntime,
  listManagedPackages,
  onPackageProgressEvent,
  removePythonRuntime,
  removeModelPackage,
  verifyPythonRuntime,
  verifyModelPackage,
} from '@/services/desktop'
import type { ManagedPackage } from '@/types'

export const usePackagesStore = defineStore('packages', () => {
  const packages = ref<ManagedPackage[]>([])
  const loading = ref(false)
  const error = ref('')

  const runtimes = computed(() => packages.value.filter((item) => item.kind === 'runtime'))
  const models = computed(() => packages.value.filter((item) => item.kind === 'model'))
  const busy = computed(() =>
    packages.value.some((item) =>
      ['queued', 'downloading', 'verifying', 'installing', 'removing'].includes(item.status),
    ),
  )

  async function refresh() {
    if (!isDesktopRuntime() || loading.value) return
    loading.value = true
    error.value = ''
    try {
      packages.value = await listManagedPackages()
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
    } finally {
      loading.value = false
    }
  }

  async function initialize() {
    await refresh()
    if (!isDesktopRuntime()) return
    await onPackageProgressEvent((event) => {
      const item = packages.value.find((candidate) => candidate.id === event.id)
      if (item) Object.assign(item, event)
    })
  }

  async function download(packageId: string) {
    error.value = ''
    try {
      const updated = await downloadManagedPackage(packageId)
      const index = packages.value.findIndex((item) => item.id === packageId)
      if (index >= 0) packages.value[index] = updated
      else packages.value.push(updated)
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  function replace(updated: ManagedPackage) {
    const index = packages.value.findIndex((item) => item.id === updated.id)
    if (index >= 0) packages.value[index] = updated
    else packages.value.push(updated)
  }

  async function installRuntime() {
    error.value = ''
    try {
      replace(await installPythonRuntime())
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  async function verifyRuntime() {
    error.value = ''
    try {
      return await verifyPythonRuntime()
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  async function removeRuntime() {
    error.value = ''
    try {
      replace(await removePythonRuntime())
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  async function installModel(packageId: string) {
    error.value = ''
    try {
      replace(await installModelPackage(packageId))
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  async function verifyModel(packageId: string) {
    error.value = ''
    try {
      return await verifyModelPackage(packageId)
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  async function removeModel(packageId: string) {
    error.value = ''
    try {
      replace(await removeModelPackage(packageId))
    } catch (reason) {
      error.value = reason instanceof Error ? reason.message : String(reason)
      await refresh()
      throw reason
    }
  }

  return {
    packages,
    runtimes,
    models,
    busy,
    loading,
    error,
    initialize,
    refresh,
    download,
    installRuntime,
    verifyRuntime,
    removeRuntime,
    installModel,
    verifyModel,
    removeModel,
  }
})
