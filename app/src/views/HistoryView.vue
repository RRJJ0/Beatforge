<script setup lang="ts">
import { computed, ref } from 'vue'
import { NButton, NCard, NIcon, NInput, NSelect, NTag, useMessage } from 'naive-ui'
import { FolderOpenOutline, RefreshOutline, SearchOutline } from '@vicons/ionicons5'

const message = useMessage()
const search = ref('')
const range = ref('all')
const history = [
  { id: 'BF-0239', title: 'Daybreak', artist: 'Mizuki', difficulty: 5.8, model: 'Standard v1.3', date: 'Today, 13:48', duration: '04:12', file: 'Mizuki - Daybreak.osz' },
  { id: 'BF-0238', title: 'Violet Pulse', artist: 'Nora Vale', difficulty: 4.2, model: 'Standard v1.3', date: 'Yesterday, 21:03', duration: '03:36', file: 'Nora Vale - Violet Pulse.osz' },
  { id: 'BF-0237', title: 'Midnight Signal', artist: 'Subframe', difficulty: 6.1, model: 'High SR v1.1', date: 'Sep 28, 18:44', duration: '05:08', file: 'Subframe - Midnight Signal.osz' },
  { id: 'BF-0236', title: 'Glass Horizon', artist: 'Ether Bloom', difficulty: 3.9, model: 'Standard v1.3', date: 'Sep 27, 10:16', duration: '03:51', file: 'Ether Bloom - Glass Horizon.osz' },
]
const filtered = computed(() => history.filter((item) => `${item.title} ${item.artist} ${item.id}`.toLowerCase().includes(search.value.toLowerCase())))
</script>

<template>
  <div class="stat-grid">
    <div class="stat-card"><div class="stat-label">Generated maps</div><div class="stat-value">24</div><div class="stat-note">Across all local sessions</div></div>
    <div class="stat-card"><div class="stat-label">Average difficulty</div><div class="stat-value">4.9★</div><div class="stat-note">Standard mode</div></div>
    <div class="stat-card"><div class="stat-label">Output storage</div><div class="stat-value">1.8 GB</div><div class="stat-note">24 .osz packages</div></div>
  </div>
  <NCard class="panel-card" :bordered="false">
    <div class="history-toolbar">
      <div><h2 class="section-title">Completed beatmaps</h2><p class="section-copy">Local generation results and exported packages.</p></div>
      <div class="toolbar-actions">
        <NInput v-model:value="search" placeholder="Search maps" clearable><template #prefix><NIcon :component="SearchOutline" /></template></NInput>
        <NSelect v-model:value="range" :options="[{ label: 'All time', value: 'all' }, { label: 'This week', value: 'week' }, { label: 'Today', value: 'today' }]" />
      </div>
    </div>
    <div class="history-table">
      <div class="table-row table-head"><span>Beatmap</span><span>Difficulty</span><span>Model</span><span>Generated</span><span></span></div>
      <div v-for="item in filtered" :key="item.id" class="table-row">
        <div class="map-cell"><div class="map-cover">{{ item.title.slice(0, 1) }}</div><div><strong>{{ item.title }}</strong><small>{{ item.artist }} · {{ item.duration }} · <span class="mono">{{ item.id }}</span></small></div></div>
        <div><NTag size="small" round>{{ item.difficulty.toFixed(1) }} ★</NTag></div>
        <span class="secondary">{{ item.model }}</span><span class="secondary">{{ item.date }}</span>
        <div class="row-actions"><NButton size="small" secondary @click="message.success(`Opening ${item.file}`)"><template #icon><NIcon :component="FolderOpenOutline" /></template>Open</NButton><NButton quaternary circle size="small" @click="message.info('Settings copied to Generate')"><template #icon><NIcon :component="RefreshOutline" /></template></NButton></div>
      </div>
      <div v-if="!filtered.length" class="empty-results">No beatmaps match “{{ search }}”.</div>
    </div>
  </NCard>
</template>

<style scoped>
.history-toolbar { display: flex; justify-content: space-between; align-items: center; gap: 20px; margin-bottom: 18px; }.toolbar-actions { display: grid; grid-template-columns: 220px 130px; gap: 10px; }
.history-table { width: 100%; }.table-row { display: grid; grid-template-columns: minmax(260px,1.4fr) 110px 160px 140px 110px; gap: 16px; align-items: center; min-height: 70px; border-top: 1px solid rgba(127,127,127,.12); font-size: 11px; }.table-head { min-height: 36px; border-top: 0; color: #858da0; font-size: 9px; text-transform: uppercase; letter-spacing: .07em; }
.map-cell { display: flex; align-items: center; gap: 11px; }.map-cover { width: 38px; height: 38px; display: grid; place-items: center; border-radius: 9px; color: #fff; font-weight: 750; background: linear-gradient(145deg,#ff66aa,#765ee9); }.map-cell strong,.map-cell small { display: block; }.map-cell strong { font-size: 12px; }.map-cell small,.secondary { margin-top: 3px; color: #858da0; font-size: 10px; }.row-actions { display: flex; justify-content: flex-end; gap: 2px; }.empty-results { padding: 54px; text-align: center; color: #858da0; }
</style>

