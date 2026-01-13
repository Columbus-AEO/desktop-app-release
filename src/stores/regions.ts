import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { usePlatformsStore } from './platforms'
import type { Country } from '@/types'
import { LOCAL_REGION } from '@/types'

export const useRegionsStore = defineStore('regions', () => {
  const commands = useTauriCommands()

  // State
  const availableCountries = ref<Country[]>([])
  const configuredRegions = ref<string[]>(['local'])
  const promptTargetRegions = ref<string[]>([])
  const platformAuthStatus = ref<Record<string, boolean>>({})
  const selectedAuthRegion = ref('local')

  // Getters
  const allRegions = computed((): Country[] => {
    const countries = availableCountries.value
      .filter(c => promptTargetRegions.value.includes(c.code.toLowerCase()))
      .map(c => ({
        code: c.code.toLowerCase(),
        name: c.name,
        flag_emoji: c.flag_emoji || ''
      }))
    return [LOCAL_REGION, ...countries]
  })

  // Actions
  async function loadAvailableCountries() {
    try {
      const countries = await commands.fetchProxyConfig()
      console.log('Loaded available countries:', countries)
      availableCountries.value = countries || []
    } catch (error) {
      const errorStr = String(error)
      if (errorStr.includes('paid plan') || errorStr.includes('Geo-targeting')) {
        console.log('Geo-targeting not available on current plan - using Local region only')
      } else {
        console.error('Failed to load countries:', error)
      }
      availableCountries.value = []
    }
  }

  async function loadConfiguredRegions() {
    try {
      // If no geo-targeting access, only allow local
      if (availableCountries.value.length === 0) {
        console.log('No geo-targeting access - using Local region only')
        configuredRegions.value = ['local']
        return
      }

      const regions = await commands.getConfiguredProxyCountries()
      if (regions && regions.length > 0) {
        const validRegions = regions.filter(r =>
          availableCountries.value.some(c => c.code.toLowerCase() === r.toLowerCase())
        )
        configuredRegions.value = ['local', ...validRegions]
      } else {
        configuredRegions.value = ['local']
      }
      console.log('Configured regions:', configuredRegions.value)
    } catch (error) {
      console.error('Failed to load regions:', error)
      configuredRegions.value = ['local']
    }
  }

  async function loadPlatformAuthStatus() {
    const platformsStore = usePlatformsStore()
    const status: Record<string, boolean> = {}

    for (const region of configuredRegions.value) {
      for (const platform of platformsStore.platformIds) {
        try {
          const authInfo = await commands.getCountryPlatformAuth(region, platform)
          const isAuth = authInfo?.is_authenticated ?? authInfo?.isAuthenticated ?? false
          status[`${region}:${platform}`] = isAuth === true
        } catch {
          status[`${region}:${platform}`] = false
        }
      }
    }

    platformAuthStatus.value = status
    console.log('Platform auth status:', platformAuthStatus.value)
  }

  async function loadPromptTargetRegions(productId: string) {
    try {
      const regions = await commands.getPromptRegions(productId)
      console.log('Loaded prompt target regions:', regions)
      promptTargetRegions.value = regions || []
    } catch (error) {
      console.log('Could not load prompt regions, defaulting to Local only:', String(error).substring(0, 100))
      promptTargetRegions.value = []
    }
  }

  function isPlatformAuthForRegion(region: string, platform: string): boolean {
    return platformAuthStatus.value[`${region}:${platform}`] === true
  }

  function hasAnyAuth(): boolean {
    return Object.values(platformAuthStatus.value).some(v => v === true)
  }

  function getAuthCountForRegion(region: string): number {
    const platformsStore = usePlatformsStore()
    return platformsStore.platformIds.filter(p => isPlatformAuthForRegion(region, p)).length
  }

  function getRegionName(code: string): string {
    if (code === 'local') return LOCAL_REGION.name
    const country = availableCountries.value.find(c => c.code.toLowerCase() === code.toLowerCase())
    return country ? country.name : code.toUpperCase()
  }

  function getRegionFlag(code: string): string {
    if (code === 'local') return LOCAL_REGION.flag_emoji
    const country = availableCountries.value.find(c => c.code.toLowerCase() === code.toLowerCase())
    return country?.flag_emoji || ''
  }

  async function addRegion(regionCode: string) {
    const platformsStore = usePlatformsStore()

    if (!configuredRegions.value.includes(regionCode)) {
      configuredRegions.value.push(regionCode)

      // Initialize auth status for new region
      for (const platform of platformsStore.platformIds) {
        platformAuthStatus.value[`${regionCode}:${platform}`] = false
      }

      // Save to storage
      await commands.setCountryPlatformAuth(regionCode, platformsStore.platformIds[0], false)
    }
  }

  async function removeRegion(regionCode: string) {
    if (regionCode === 'local') return

    const platformsStore = usePlatformsStore()
    configuredRegions.value = configuredRegions.value.filter(r => r !== regionCode)

    // Remove from auth status
    for (const platform of platformsStore.platformIds) {
      delete platformAuthStatus.value[`${regionCode}:${platform}`]
    }
  }

  async function setAuthStatus(region: string, platform: string, authenticated: boolean) {
    const key = `${region}:${platform}`
    platformAuthStatus.value[key] = authenticated
    await commands.setPlatformAuthStatus(region, platform, authenticated)
  }

  function selectRegion(region: string) {
    selectedAuthRegion.value = region
  }

  return {
    // State
    availableCountries,
    configuredRegions,
    promptTargetRegions,
    platformAuthStatus,
    selectedAuthRegion,

    // Getters
    allRegions,

    // Actions
    loadAvailableCountries,
    loadConfiguredRegions,
    loadPlatformAuthStatus,
    loadPromptTargetRegions,
    isPlatformAuthForRegion,
    hasAnyAuth,
    getAuthCountForRegion,
    getRegionName,
    getRegionFlag,
    addRegion,
    removeRegion,
    setAuthStatus,
    selectRegion
  }
})
