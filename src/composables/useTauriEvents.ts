import { listen } from '@tauri-apps/api/event'
import { useScanStore } from '@/stores/scan'
import { useRegionsStore } from '@/stores/regions'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'
import { useProductsStore } from '@/stores/products'
import type { ScanProgress, ScanComplete, PaaProgress } from '@/types'

// Track listeners to prevent duplicates
let listenersSetup = false

export function useTauriEvents() {
  async function setupEventListeners() {
    if (listenersSetup) {
      console.log('[Events] Listeners already set up, skipping')
      return
    }

    const scanStore = useScanStore()
    const regionsStore = useRegionsStore()
    const authStore = useAuthStore()
    const uiStore = useUiStore()
    const productsStore = useProductsStore()

    // Scan started (for auto-scans or external triggers)
    await listen<{ productId: string; totalPrompts: number; platforms: string[] }>(
      'scan:started',
      (event) => {
        console.log('[Events] Scan started:', event.payload)
        // Only switch view if we're not already scanning
        if (!scanStore.isScanning) {
          scanStore.isScanning = true
          scanStore.resetProgress()
          uiStore.showView('scanning')
        }
        // Refresh quota since scan is consuming tests
        productsStore.loadDailyUsage()
      }
    )

    // Scan progress
    await listen<ScanProgress>('scan:progress', (event) => {
      scanStore.updateProgress(event.payload)
    })

    // Scan complete
    await listen<ScanComplete>('scan:complete', (event) => {
      scanStore.handleComplete(event.payload)
    })

    // Scan error
    await listen<{ message?: string } | string>('scan:error', (event) => {
      scanStore.handleError(event.payload)
    })

    // Scan countdown
    await listen<number>('scan:countdown', (event) => {
      scanStore.setCountdown(event.payload)
    })

    // Platform auth changed (from webviews)
    await listen<{ region: string; platform: string; authenticated: boolean }>(
      'platform-auth-changed',
      async (event) => {
        const { region, platform, authenticated } = event.payload
        regionsStore.platformAuthStatus[`${region}:${platform}`] = authenticated
      }
    )

    // OAuth login success
    await listen('auth:success', async () => {
      console.log('OAuth login successful, refreshing app state...')
      await authStore.checkAuthStatus()
    })

    // PAA progress (handled by components directly if needed)
    await listen<PaaProgress>('paa:progress', (event) => {
      console.log('[Keyword Discovery] Progress:', event.payload)
      // This can be handled by the KeywordDiscoveryModal component
    })

    listenersSetup = true
    console.log('[Events] All Tauri event listeners set up')
  }

  return {
    setupEventListeners
  }
}
