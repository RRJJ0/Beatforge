<script setup lang="ts">
import { computed, h } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import {
  NButton, NConfigProvider, NGlobalStyle, NIcon, NLayout, NLayoutContent,
  NLayoutSider, NMenu, NMessageProvider, NTag, darkTheme,
  type GlobalThemeOverrides, type MenuOption,
} from 'naive-ui'
import {
  AlbumsOutline, HardwareChipOutline, MoonOutline, OptionsOutline,
  SettingsOutline, SparklesOutline, SunnyOutline, TimeOutline,
} from '@vicons/ionicons5'
import { useAppStore } from '@/stores/app'

const route = useRoute()
const router = useRouter()
const appStore = useAppStore()
const icon = (component: typeof SparklesOutline) => () => h(NIcon, null, { default: () => h(component) })
const menuOptions: MenuOption[] = [
  { label: 'Generate', key: '/', icon: icon(SparklesOutline) },
  { label: 'Jobs', key: '/jobs', icon: icon(OptionsOutline) },
  { label: 'History', key: '/history', icon: icon(TimeOutline) },
  { label: 'Models', key: '/models', icon: icon(AlbumsOutline) },
  { label: 'Settings', key: '/settings', icon: icon(SettingsOutline) },
]
const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#ff66aa', primaryColorHover: '#ff83bd', primaryColorPressed: '#e84f94',
    primaryColorSuppl: '#ff66aa', borderRadius: '10px', borderRadiusSmall: '8px',
    fontFamily: 'Inter, "Segoe UI", system-ui, sans-serif',
  },
  Button: { fontWeight: '650' }, Card: { borderRadius: '14px' }, Menu: { itemBorderRadius: '10px' },
}
const currentPath = computed(() => route.path)
const pageTitle = computed(() => String(route.meta.title ?? 'Generate'))
const pageDescription = computed(() => String(route.meta.description ?? 'Create a playable beatmap from your music.'))
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
          <NMenu :value="currentPath" :options="menuOptions" :indent="18" @update:value="(path) => router.push(String(path))" />
          <div class="sidebar-footer">
            <div class="runtime-card">
              <div class="runtime-heading"><NIcon :component="HardwareChipOutline" /><span>Compute engine</span></div>
              <strong>RTX 4070 · Ready</strong><small>CUDA 12.4 · 7.2 GB free</small>
              <div class="runtime-meter"><span /></div>
            </div>
            <div class="version-row"><span>Local workspace</span><span>v0.1.0</span></div>
          </div>
        </NLayoutSider>

        <NLayoutContent class="workspace" :native-scrollbar="false">
          <header class="topbar">
            <div>
              <div class="eyebrow">WORKSPACE / {{ pageTitle.toUpperCase() }}</div>
              <h1>{{ pageTitle }}</h1><p>{{ pageDescription }}</p>
            </div>
            <div class="topbar-actions">
              <NTag round size="small" type="success">Engine ready</NTag>
              <NButton quaternary circle aria-label="Toggle color mode" @click="appStore.toggleTheme">
                <template #icon><NIcon :component="appStore.isDark ? SunnyOutline : MoonOutline" /></template>
              </NButton>
            </div>
          </header>
          <main class="page-content"><RouterView /></main>
        </NLayoutContent>
      </NLayout>
    </NMessageProvider>
  </NConfigProvider>
</template>

