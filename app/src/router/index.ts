import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', name: 'generate', component: () => import('@/views/GenerateView.vue'), meta: { title: 'Generate', description: 'Create a playable beatmap from your music.' } },
    { path: '/jobs', name: 'jobs', component: () => import('@/views/JobsView.vue'), meta: { title: 'Jobs', description: 'Monitor generation progress and queued work.' } },
    { path: '/history', name: 'history', component: () => import('@/views/HistoryView.vue'), meta: { title: 'History', description: 'Review, open, and reuse completed beatmaps.' } },
    { path: '/models', name: 'models', component: () => import('@/views/ModelsView.vue'), meta: { title: 'Models', description: 'Manage local Mapperatorinator model packages.' } },
    { path: '/settings', name: 'settings', component: () => import('@/views/SettingsView.vue'), meta: { title: 'Settings', description: 'Configure local paths, compute, and output defaults.' } },
  ],
})
export default router

