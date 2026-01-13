import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { DEFAULT_PLATFORMS, DEFAULT_PLATFORM_URLS, DEFAULT_PLATFORM_NAMES, capitalizeFirst } from '@/utils/constants'
import type { Platform } from '@/types'

export const usePlatformsStore = defineStore('platforms', () => {
  const commands = useTauriCommands()

  // State
  const platforms = ref<Platform[]>([])
  const isLoaded = ref(false)

  // Derived maps
  const platformIds = computed(() => platforms.value.map(p => p.id))
  const platformUrls = computed(() => {
    const map: Record<string, string> = {}
    platforms.value.forEach(p => {
      map[p.id] = p.website_url || ''
    })
    return map
  })
  const platformNames = computed(() => {
    const map: Record<string, string> = {}
    platforms.value.forEach(p => {
      map[p.id] = p.name || p.id
    })
    return map
  })
  const platformLogos = computed(() => {
    const map: Record<string, string | null> = {}
    platforms.value.forEach(p => {
      map[p.id] = p.logo_url || null
    })
    return map
  })

  // Actions
  async function loadPlatforms() {
    try {
      const data = await commands.getAiPlatforms(false)
      console.log('Loaded platforms:', data)
      platforms.value = data
      isLoaded.value = true
    } catch (error) {
      console.error('Failed to load platforms:', error)
      // Use defaults
      platforms.value = DEFAULT_PLATFORMS.map(id => ({
        id,
        name: DEFAULT_PLATFORM_NAMES[id] || capitalizeFirst(id),
        website_url: DEFAULT_PLATFORM_URLS[id] || '',
        logo_url: null
      }))
      isLoaded.value = true
    }
  }

  function getPlatformName(id: string): string {
    return platformNames.value[id] || capitalizeFirst(id)
  }

  function getPlatformLogo(id: string): string | null {
    return platformLogos.value[id] || null
  }

  function getPlatformUrl(id: string): string {
    return platformUrls.value[id] || ''
  }

  return {
    // State
    platforms,
    isLoaded,
    platformIds,
    platformUrls,
    platformNames,
    platformLogos,

    // Actions
    loadPlatforms,
    getPlatformName,
    getPlatformLogo,
    getPlatformUrl
  }
})
