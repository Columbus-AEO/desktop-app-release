import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useUiStore } from './ui'
import { useProductsStore } from './products'
import { usePlatformsStore } from './platforms'
import { useRegionsStore } from './regions'
import type { ScanPhase, PlatformState, ScanComplete } from '@/types'
import { PHASE_DISPLAY_TEXT } from '@/utils/constants'

export const useScanStore = defineStore('scan', () => {
  const commands = useTauriCommands()

  // State
  const isScanning = ref(false)
  const isStarting = ref(false)
  const phase = ref<ScanPhase>('initializing')
  const progress = ref(0)
  const countdownSeconds = ref<number | null>(null)
  const platforms = ref<Record<string, PlatformState>>({})
  const lastResult = ref<ScanComplete | null>(null)

  // Getters
  const phaseDisplayText = computed(() =>
    PHASE_DISPLAY_TEXT[phase.value] || phase.value || 'Processing...'
  )

  // Actions
  function resetProgress() {
    const platformsStore = usePlatformsStore()
    progress.value = 0
    phase.value = 'initializing'
    countdownSeconds.value = null
    lastResult.value = null

    // Initialize platform states
    const states: Record<string, PlatformState> = {}
    platformsStore.platformIds.forEach(id => {
      states[id] = {
        status: 'pending',
        total: 0,
        submitted: 0,
        collected: 0
      }
    })
    platforms.value = states
  }

  async function startScan() {
    // Prevent double-clicks
    if (isStarting.value || isScanning.value) return

    const uiStore = useUiStore()
    const productsStore = useProductsStore()
    const platformsStore = usePlatformsStore()
    const regionsStore = useRegionsStore()

    if (!productsStore.selectedProductId) return

    if (!regionsStore.hasAnyAuth()) {
      uiStore.showAuthRequiredModal([])
      return
    }

    isStarting.value = true

    // Refresh usage data
    await productsStore.loadDailyUsage()
    await productsStore.loadProductPromptCount(productsStore.selectedProductId)

    const availableTests = productsStore.availableTests

    // Check quota
    if (!productsStore.dailyUsage.isUnlimited && availableTests <= 0) {
      if (productsStore.dailyUsage.pendingEvaluations > 0) {
        uiStore.showMessage(
          `Please wait - ${productsStore.dailyUsage.pendingEvaluations} tests are pending evaluation`,
          'Cannot Start Scan',
          'warning'
        )
      } else {
        uiStore.showMessage(
          `Daily limit reached (${productsStore.dailyUsage.current}/${productsStore.dailyUsage.limit}). Resets at midnight.`,
          'Cannot Start Scan',
          'warning'
        )
      }
      isStarting.value = false
      return
    }

    // Check for prompts
    if (productsStore.promptCount === 0) {
      uiStore.showMessage(
        'No prompts configured for this product. Add prompts in the dashboard.',
        'No Prompts',
        'warning'
      )
      isStarting.value = false
      return
    }

    // Check which platforms are needed
    try {
      const promptRegions = await commands.getPromptTargetRegions(productsStore.selectedProductId)
      const neededRegions = new Set<string>()

      for (const regions of Object.values(promptRegions)) {
        if (regions.length === 0) {
          neededRegions.add('local')
        } else {
          regions.forEach(r => neededRegions.add(r.toLowerCase()))
        }
      }

      // Check auth for needed regions
      const missingAuth: { region: string; platforms: string[] }[] = []
      for (const region of neededRegions) {
        const hasAuthForRegion = platformsStore.platformIds.some(p =>
          regionsStore.isPlatformAuthForRegion(region, p)
        )
        if (!hasAuthForRegion) {
          missingAuth.push({ region, platforms: platformsStore.platformIds })
        }
      }

      if (missingAuth.length > 0) {
        uiStore.showAuthRequiredModal(missingAuth)
        isStarting.value = false
        return
      }

      // Get authenticated platforms
      let authPlatforms = platformsStore.platformIds.filter(p =>
        regionsStore.configuredRegions.some(r => regionsStore.isPlatformAuthForRegion(r, p))
      )

      if (authPlatforms.length === 0) {
        uiStore.showAuthRequiredModal([])
        isStarting.value = false
        return
      }

      const samples = productsStore.productConfig?.samples_per_prompt || 1
      const maxTestsToUse = productsStore.promptCount * samples * authPlatforms.length

      // Calculate limited platforms if needed
      let limitedPlatforms = authPlatforms
      if (!productsStore.dailyUsage.isUnlimited && maxTestsToUse > availableTests) {
        const testsPerPrompt = productsStore.promptCount * samples
        const platformsWeCanAfford = Math.floor(availableTests / testsPerPrompt)

        if (platformsWeCanAfford > 0 && platformsWeCanAfford < authPlatforms.length) {
          limitedPlatforms = authPlatforms.slice(0, platformsWeCanAfford)
          console.log(`Limiting scan to ${platformsWeCanAfford} platforms due to quota`)
        }
      }

      // Start scan
      isScanning.value = true
      resetProgress()
      uiStore.showView('scanning')

      await commands.startScan({
        productId: productsStore.selectedProductId,
        samplesPerPrompt: samples,
        platforms: limitedPlatforms,
        maxTests: productsStore.dailyUsage.isUnlimited ? null : availableTests
      })

      console.log('Scan started with platforms:', limitedPlatforms)
      isStarting.value = false
    } catch (error) {
      console.error('Start scan error:', error)
      uiStore.showMessage('Failed to start scan: ' + error, 'Error', 'error')
      uiStore.showView('main')
      isScanning.value = false
      isStarting.value = false
    }
  }

  async function cancelScan() {
    const uiStore = useUiStore()
    try {
      await commands.cancelScan()
      isScanning.value = false
      uiStore.showView('main')
    } catch (error) {
      console.error('Cancel scan error:', error)
    }
  }

  function updateProgress(progressData: {
    phase?: ScanPhase
    platforms?: Record<string, PlatformState>
    countdownSeconds?: number | null
  }) {
    if (!progressData) return

    if (progressData.phase) {
      phase.value = progressData.phase
    }

    if (progressData.platforms) {
      platforms.value = progressData.platforms

      // Calculate overall progress
      let totalSubmitted = 0
      let totalCollected = 0
      let totalTasks = 0

      for (const state of Object.values(progressData.platforms)) {
        totalSubmitted += state.submitted || 0
        totalCollected += state.collected || 0
        totalTasks += state.total || 0
      }

      if (totalTasks > 0) {
        const submissionProgress = (totalSubmitted / totalTasks) * 50
        const collectionProgress = (totalCollected / totalTasks) * 50
        progress.value = Math.round(submissionProgress + collectionProgress)
      }
    }

    if (progressData.countdownSeconds !== undefined) {
      countdownSeconds.value = progressData.countdownSeconds
    }
  }

  function setCountdown(seconds: number) {
    countdownSeconds.value = seconds
  }

  async function handleComplete(result: ScanComplete) {
    const uiStore = useUiStore()
    const productsStore = useProductsStore()

    console.log('Scan complete:', result)
    isScanning.value = false
    lastResult.value = result

    uiStore.showView('complete')
    await productsStore.loadScheduleInfo()
    await productsStore.loadDailyUsage()
  }

  function handleError(error: { message?: string } | string) {
    const uiStore = useUiStore()
    isScanning.value = false

    const errorMsg = typeof error === 'string' ? error : error.message || String(error)
    const isCancelled = errorMsg.toLowerCase().includes('cancel')

    uiStore.showMessage(
      errorMsg,
      isCancelled ? 'Scan Cancelled' : 'Scan Error',
      isCancelled ? 'info' : 'error'
    )
    uiStore.showView('main')
  }

  return {
    // State
    isScanning,
    isStarting,
    phase,
    progress,
    countdownSeconds,
    platforms,
    lastResult,

    // Getters
    phaseDisplayText,

    // Actions
    resetProgress,
    startScan,
    cancelScan,
    updateProgress,
    setCountdown,
    handleComplete,
    handleError
  }
})
