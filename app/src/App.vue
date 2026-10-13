<script setup lang="ts">
import { computed, h, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NConfigProvider,
  NGlobalStyle,
  NIcon,
  NLayout,
  NLayoutContent,
  NLayoutSider,
  NMenu,
  NMessageProvider,
  NTag,
  darkTheme,
  type GlobalThemeOverrides,
  type MenuOption,
} from 'naive-ui'
import {
  AlbumsOutline,
  CloseOutline,
  HardwareChipOutline,
  MoonOutline,
  OptionsOutline,
  SettingsOutline,
  SparklesOutline,
  SunnyOutline,
  TimeOutline,
} from '@vicons/ionicons5'
import { useAppStore } from '@/stores/app'
import { useDesktopStore } from '@/stores/desktop'
import { useJobsStore } from '@/stores/jobs'
import { loadDesktopSettings } from '@/services/desktop'
import { onDesktopNotificationAction, sendJobSystemNotification } from '@/services/notifications'
import {
  TerminalJobTracker,
  shouldSendSystemNotification,
  type JobNotice,
} from '@/utils/jobNotifications'
import type { GenerationJob, JobStatus } from '@/types'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const desktopStore = useDesktopStore()
const jobsStore = useJobsStore()
const jobNotice = ref<JobNotice | null>(null)
const notificationTracker = new TerminalJobTracker()
let unsubscribeJobEvents: (() => void) | undefined
let unsubscribeNotificationActions: (() => void) | undefined

async function openNotifiedJob(jobId: string) {
  jobNotice.value = null
  await router.push({ name: 'jobs', query: { job: jobId } })
}

async function handleJobEvent(job: GenerationJob, previousStatus?: JobStatus) {
  const notice = notificationTracker.consume(previousStatus, job)
  if (!notice) return
  jobNotice.value = notice
  if (!shouldSendSystemNotification(true, document.visibilityState, document.hasFocus())) return
  try {
    const settings = await loadDesktopSettings()
    if (
      shouldSendSystemNotification(
        settings.desktopNotifications,
        document.visibilityState,
        document.hasFocus(),
      )
    ) {
      await sendJobSystemNotification(notice)
    }
  } catch {
    // The in-app notice remains available when settings or native notifications fail.
  }
}

onMounted(async () => {
  unsubscribeJobEvents = jobsStore.subscribeJobEvents((job, previousStatus) => {
    void handleJobEvent(job, previousStatus)
  })
  unsubscribeNotificationActions = await onDesktopNotificationAction((jobId) => {
    void openNotifiedJob(jobId)
  })
  await Promise.allSettled([desktopStore.initialize(), jobsStore.initialize()])
})

onBeforeUnmount(() => {
  unsubscribeJobEvents?.()
  unsubscribeNotificationActions?.()
})
const icon = (component: typeof SparklesOutline) => () =>
  h(NIcon, null, { default: () => h(component) })
const menuOptions: MenuOption[] = [
  { label: 'Generate', key: '/', icon: icon(SparklesOutline) },
  { label: 'Jobs', key: '/jobs', icon: icon(OptionsOutline) },
  { label: 'History', key: '/history', icon: icon(TimeOutline) },
  { label: 'Models', key: '/models', icon: icon(AlbumsOutline) },
  { label: 'Settings', key: '/settings', icon: icon(SettingsOutline) },
]
const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#ff66aa',
    primaryColorHover: '#ff83bd',
    primaryColorPressed: '#e84f94',
    primaryColorSuppl: '#ff66aa',
    borderRadius: '10px',
    borderRadiusSmall: '8px',
    fontFamily: 'Inter, "Segoe UI", system-ui, sans-serif',
  },
  Button: { fontWeight: '650' },
  Card: { borderRadius: '14px' },
  Menu: { itemBorderRadius: '10px' },
}
const currentPath = computed(() => route.path)
const pageTitle = computed(() => String(route.meta.title ?? 'Generate'))
const pageDescription = computed(() =>
  String(route.meta.description ?? 'Create a playable beatmap from your music.'),
)
</script>

<template>
  <NConfigProvider :theme="appStore.isDark ? darkTheme : null" :theme-overrides="themeOverrides">
    <NGlobalStyle />
    <NMessageProvider placement="top-right">
      <NLayout class="app-shell" :class="{ 'is-dark': appStore.isDark }" has-sider>
        <NLayoutSider bordered class="app-sidebar" :width="236" :native-scrollbar="false">
          <div class="brand">
            <div class="brand-mark"><span>●</span></div>
            <div><strong>BEATFORGE</strong><small>AI beatmap studio</small></div>
          </div>
          <NMenu
            :value="currentPath"
            :options="menuOptions"
            :indent="18"
            @update:value="(path) => router.push(String(path))"
          />
          <div class="sidebar-footer">
            <div class="runtime-card">
              <div class="runtime-heading">
                <NIcon :component="HardwareChipOutline" /><span>Compute engine</span>
              </div>
              <strong>{{ desktopStore.engineLabel }}</strong
              ><small>{{ desktopStore.engineDetail }}</small>
              <div class="runtime-meter"><span /></div>
            </div>
            <div class="version-row"><span>Local workspace</span><span>v0.1.0</span></div>
          </div>
        </NLayoutSider>

        <NLayoutContent class="workspace" :native-scrollbar="false">
          <header class="topbar">
            <div>
              <div class="eyebrow">WORKSPACE / {{ pageTitle.toUpperCase() }}</div>
              <h1>{{ pageTitle }}</h1>
              <p>{{ pageDescription }}</p>
            </div>
            <div class="topbar-actions">
              <NTag round size="small" type="success">Engine ready</NTag>
              <NButton
                quaternary
                circle
                aria-label="Toggle color mode"
                @click="appStore.toggleTheme"
              >
                <template #icon
                  ><NIcon :component="appStore.isDark ? SunnyOutline : MoonOutline"
                /></template>
              </NButton>
            </div>
          </header>
          <div v-if="jobNotice" class="job-notice" :class="`is-${jobNotice.kind}`" role="status">
            <button
              type="button"
              class="job-notice-content"
              @click="openNotifiedJob(jobNotice.jobId)"
            >
              <strong>{{ jobNotice.title }}</strong
              ><small>{{ jobNotice.body }}</small>
            </button>
            <NButton quaternary circle aria-label="Dismiss notification" @click="jobNotice = null">
              <template #icon><NIcon :component="CloseOutline" /></template>
            </NButton>
          </div>
          <main class="page-content"><RouterView /></main>
        </NLayoutContent>
      </NLayout>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style scoped>
.job-notice {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 28px 14px;
  padding: 8px 10px 8px 14px;
  border: 1px solid rgba(127, 127, 127, 0.18);
  border-left: 3px solid #ff66aa;
  border-radius: 10px;
  background: rgba(127, 127, 127, 0.06);
}
.job-notice.is-warning {
  border-left-color: #f0a020;
}
.job-notice.is-error {
  border-left-color: #d03050;
}
.job-notice-content {
  flex: 1;
  padding: 0;
  text-align: left;
  color: inherit;
  border: 0;
  background: transparent;
  cursor: pointer;
}
.job-notice-content strong,
.job-notice-content small {
  display: block;
}
.job-notice-content small {
  margin-top: 2px;
  color: #858da0;
}
</style>
