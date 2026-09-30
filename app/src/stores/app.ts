import { defineStore } from 'pinia'
import { ref } from 'vue'
export const useAppStore = defineStore('app', () => {
  const isDark = ref(localStorage.getItem('beatforge:theme') !== 'light')
  function toggleTheme() {
    isDark.value = !isDark.value
    localStorage.setItem('beatforge:theme', isDark.value ? 'dark' : 'light')
  }
  return { isDark, toggleTheme }
})

