import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { usePlatformsStore } from './platforms'
import { useRegionsStore } from './regions'
import type { Product, ProductConfig, DailyUsage, ScheduleInfo, Organization } from '@/types'

export const useProductsStore = defineStore('products', () => {
  const commands = useTauriCommands()

  // State
  const products = ref<Product[]>([])
  const organizations = ref<Organization[]>([])
  const selectedProductId = ref<string | null>(null)
  const productConfig = ref<ProductConfig | null>(null)
  const promptCount = ref(0)
  const dailyUsage = ref<DailyUsage>({
    current: 0,
    limit: 30,
    remaining: 30,
    effectiveRemaining: 30,
    pendingEvaluations: 0,
    isUnlimited: false,
    plan: 'free'
  })
  const scheduleInfo = ref<ScheduleInfo | null>(null)
  const autostartEnabled = ref(false)
  const isLoading = ref(false)

  // Getters
  const selectedProduct = computed(() =>
    products.value.find(p => p.id === selectedProductId.value) || null
  )

  const hasMultipleOrgs = computed(() => {
    const uniqueOrgIds = new Set(products.value.map(p => p.organizationId))
    return uniqueOrgIds.size > 1
  })

  const productsByOrg = computed(() => {
    const map: Record<string, { name: string; products: Product[] }> = {}
    products.value.forEach(p => {
      if (!map[p.organizationId]) {
        map[p.organizationId] = {
          name: p.organizationName || 'Unknown',
          products: []
        }
      }
      map[p.organizationId].products.push(p)
    })
    return map
  })

  const availableTests = computed(() =>
    dailyUsage.value.effectiveRemaining ?? dailyUsage.value.remaining
  )

  // Actions
  async function loadProducts() {
    isLoading.value = true
    try {
      const status = await commands.getStatus()
      products.value = status.products || []
      organizations.value = status.organizations || []
      console.log('Loaded products:', products.value)

      // Restore last selected product
      const lastProductId = await commands.getLastProductId()
      if (lastProductId && products.value.find(p => p.id === lastProductId)) {
        await selectProduct(lastProductId)
      }
    } catch (error) {
      console.error('Failed to load products:', error)
    } finally {
      isLoading.value = false
    }
  }

  async function selectProduct(productId: string) {
    selectedProductId.value = productId
    await commands.setLastProductId(productId)
    await loadProductConfig(productId)

    const regionsStore = useRegionsStore()
    await regionsStore.loadPromptTargetRegions(productId)
    await loadDailyUsage()
    await loadProductPromptCount(productId)
    await loadScheduleInfo()
  }

  async function loadProductConfig(productId: string) {
    try {
      const config = await commands.getProductConfig(productId)
      console.log('Loaded product config:', config)
      productConfig.value = config

      // Auto-initialize platforms if empty
      const platformsStore = usePlatformsStore()
      if (!config.ready_platforms || config.ready_platforms.length === 0) {
        console.log('Product has no platforms configured, initializing:', platformsStore.platformIds)
        if (platformsStore.platformIds.length > 0) {
          await saveProductConfig(true)
        }
      }
    } catch (error) {
      console.error('Failed to load product config:', error)
    }
  }

  async function saveProductConfig(force = false) {
    if (!selectedProductId.value || (!force && isLoading.value)) return

    const platformsStore = usePlatformsStore()
    const config = productConfig.value

    try {
      await commands.setProductConfig(
        selectedProductId.value,
        platformsStore.platformIds,
        config?.samples_per_prompt || 1,
        config?.auto_run_enabled ?? true,
        config?.scans_per_day || 1,
        config?.time_window_start ?? 9,
        config?.time_window_end ?? 17
      )
      console.log('Saved product config')
    } catch (error) {
      console.error('Failed to save product config:', error)
    }
  }

  async function loadDailyUsage() {
    try {
      const usage = await commands.checkDailyUsage()
      dailyUsage.value = {
        current: usage.current || 0,
        limit: usage.limit || 5,
        remaining: usage.remaining || 0,
        effectiveRemaining: usage.effectiveRemaining ?? usage.remaining ?? 30,
        pendingEvaluations: usage.pendingEvaluations ?? 0,
        isUnlimited: usage.isUnlimited || usage.limit === -1,
        plan: usage.plan || 'free'
      }
      console.log('Daily usage loaded:', dailyUsage.value)
    } catch (error) {
      console.log('Failed to load daily usage:', error)
    }
  }

  async function loadProductPromptCount(productId: string) {
    try {
      const promptData = await commands.fetchExtensionPrompts(productId)
      promptCount.value = promptData?.totalPrompts || promptData?.prompts?.length || 0
      console.log('Product prompt count:', promptCount.value)

      // Also update daily usage from quota if available
      if (promptData?.quota) {
        const baseRemaining = promptData.quota.promptsRemaining ??
          (promptData.quota.promptsPerDay - promptData.quota.promptsUsedToday)
        dailyUsage.value = {
          current: promptData.quota.promptsUsedToday || 0,
          limit: promptData.quota.promptsPerDay || 30,
          remaining: baseRemaining,
          effectiveRemaining: promptData.quota.effectiveRemaining ?? baseRemaining,
          pendingEvaluations: promptData.quota.pendingEvaluations ?? 0,
          isUnlimited: promptData.quota.isUnlimited || promptData.quota.promptsPerDay === -1,
          plan: promptData.quota.plan || 'free'
        }
      }
    } catch (error) {
      console.log('Failed to load prompt count:', error)
      promptCount.value = 0
    }
  }

  async function loadScheduleInfo() {
    if (!selectedProductId.value) return
    try {
      const info = await commands.getScheduleInfo(selectedProductId.value)
      scheduleInfo.value = info
      console.log('[Schedule] Info:', info)
    } catch (error) {
      console.error('Failed to load schedule info:', error)
    }
  }

  async function loadAutostartSetting() {
    try {
      autostartEnabled.value = await commands.getAutostartEnabled()
    } catch (error) {
      console.error('Failed to load autostart setting:', error)
    }
  }

  async function setAutostartEnabled(enabled: boolean) {
    try {
      await commands.setAutostartEnabled(enabled)
      autostartEnabled.value = enabled
    } catch (error) {
      console.error('Failed to save autostart setting:', error)
    }
  }

  function updateConfig(updates: Partial<ProductConfig>) {
    if (productConfig.value) {
      productConfig.value = { ...productConfig.value, ...updates }
    }
  }

  return {
    // State
    products,
    organizations,
    selectedProductId,
    productConfig,
    promptCount,
    dailyUsage,
    scheduleInfo,
    autostartEnabled,
    isLoading,

    // Getters
    selectedProduct,
    hasMultipleOrgs,
    productsByOrg,
    availableTests,

    // Actions
    loadProducts,
    selectProduct,
    loadProductConfig,
    saveProductConfig,
    loadDailyUsage,
    loadProductPromptCount,
    loadScheduleInfo,
    loadAutostartSetting,
    setAutostartEnabled,
    updateConfig
  }
})
