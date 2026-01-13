import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useUiStore } from './ui'
import { useProductsStore } from './products'
import { usePlatformsStore } from './platforms'
import { useRegionsStore } from './regions'
import { initializeRealtime, disconnectRealtime } from '@/composables/useSupabaseRealtime'
import type { User } from '@/types'

export const useAuthStore = defineStore('auth', () => {
  const commands = useTauriCommands()

  // State
  const currentUser = ref<User | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // Getters
  const isAuthenticated = computed(() => currentUser.value !== null)
  const userEmail = computed(() => currentUser.value?.email || '')

  // Actions
  async function login(email: string, password: string) {
    isLoading.value = true
    error.value = null

    try {
      const result = await commands.login(email, password)
      currentUser.value = result.user
      await checkAuthStatus()
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function loginWithGoogle() {
    isLoading.value = true
    error.value = null

    try {
      const user = await commands.loginWithGoogle()
      currentUser.value = user
      await checkAuthStatus()
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function logout() {
    try {
      // Disconnect realtime subscriptions
      await disconnectRealtime()
      await commands.logout()
      currentUser.value = null
      const uiStore = useUiStore()
      uiStore.showView('login')
    } catch (e) {
      console.error('Logout error:', e)
    }
  }

  async function checkAuthStatus() {
    const uiStore = useUiStore()
    const productsStore = useProductsStore()
    const platformsStore = usePlatformsStore()
    const regionsStore = useRegionsStore()

    try {
      const status = await commands.getAuthStatus()
      console.log('Auth status:', status)

      if (status.authenticated && status.user) {
        currentUser.value = status.user

        // Load data in parallel
        await Promise.all([
          platformsStore.loadPlatforms(),
          productsStore.loadProducts(),
          regionsStore.loadAvailableCountries()
        ])

        // Load regions and auth status (depend on above)
        await regionsStore.loadConfiguredRegions()
        await regionsStore.loadPlatformAuthStatus()

        // Initialize Supabase realtime for reactive updates
        const firstProduct = productsStore.products[0]
        if (firstProduct?.organizationId) {
          try {
            await initializeRealtime(firstProduct.organizationId)
          } catch (e) {
            console.warn('[Auth] Failed to initialize realtime:', e)
          }
        }

        // Check if scan is running
        const scanRunning = await commands.isScanRunning()
        if (scanRunning) {
          uiStore.showView('scanning')
        } else if (!await commands.isOnboardingCompleted()) {
          uiStore.showView('onboarding')
        } else {
          uiStore.showView('main')
        }
      } else {
        uiStore.showView('login')
      }
    } catch (e) {
      console.error('Auth check failed:', e)
      uiStore.showView('login')
    }
  }

  return {
    // State
    currentUser,
    isLoading,
    error,

    // Getters
    isAuthenticated,
    userEmail,

    // Actions
    login,
    loginWithGoogle,
    logout,
    checkAuthStatus
  }
})
