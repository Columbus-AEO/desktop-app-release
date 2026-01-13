import { createClient, RealtimeChannel } from '@supabase/supabase-js'
import { ref, onUnmounted } from 'vue'
import { useProductsStore } from '@/stores/products'
import { SUPABASE_URL, SUPABASE_ANON_KEY } from '@/utils/constants'

// Create Supabase client
const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY)

// Track active subscriptions
let promptResultsChannel: RealtimeChannel | null = null

// Debounce timer for quota refresh
let refreshTimer: ReturnType<typeof setTimeout> | null = null

export function useSupabaseRealtime() {
  const isConnected = ref(false)

  async function setupRealtimeSubscriptions(organizationId: string) {
    const productsStore = useProductsStore()

    // Clean up existing subscriptions
    await cleanupSubscriptions()

    console.log('[Realtime] Setting up subscriptions for org:', organizationId)

    // Subscribe to prompt_results for:
    // - INSERT: new results = quota consumed
    // - UPDATE: evaluation completed = pending count decreases
    // This table already has realtime enabled (migration 060)
    promptResultsChannel = supabase
      .channel(`prompt-results-${organizationId}`)
      .on(
        'postgres_changes',
        {
          event: 'INSERT',
          schema: 'public',
          table: 'prompt_results',
          filter: `organization_id=eq.${organizationId}`
        },
        (payload) => {
          console.log('[Realtime] New prompt result (quota consumed):', payload.new)
          // Debounce refresh to avoid multiple calls during batch inserts
          debouncedRefresh(productsStore)
        }
      )
      .on(
        'postgres_changes',
        {
          event: 'UPDATE',
          schema: 'public',
          table: 'prompt_results',
          filter: `organization_id=eq.${organizationId}`
        },
        (payload) => {
          const newRecord = payload.new as { status?: string }
          // Check if this is an evaluation completion
          if (newRecord.status === 'evaluated') {
            console.log('[Realtime] Evaluation completed, refreshing quota')
            debouncedRefresh(productsStore)
          }
        }
      )
      .subscribe((status) => {
        console.log('[Realtime] Prompt results channel status:', status)
        isConnected.value = status === 'SUBSCRIBED'
      })

    console.log('[Realtime] Subscriptions established')
  }

  function debouncedRefresh(productsStore: ReturnType<typeof useProductsStore>) {
    // Clear existing timer
    if (refreshTimer) {
      clearTimeout(refreshTimer)
    }
    // Wait 500ms before refreshing to batch multiple changes
    refreshTimer = setTimeout(() => {
      productsStore.loadDailyUsage()
      refreshTimer = null
    }, 500)
  }

  async function cleanupSubscriptions() {
    if (promptResultsChannel) {
      await supabase.removeChannel(promptResultsChannel)
      promptResultsChannel = null
    }
    if (refreshTimer) {
      clearTimeout(refreshTimer)
      refreshTimer = null
    }
    isConnected.value = false
    console.log('[Realtime] Subscriptions cleaned up')
  }

  // Cleanup on unmount
  onUnmounted(() => {
    cleanupSubscriptions()
  })

  return {
    isConnected,
    setupRealtimeSubscriptions,
    cleanupSubscriptions,
    supabase
  }
}

// Singleton pattern for app-wide realtime connection
let realtimeSetup = false

export async function initializeRealtime(organizationId: string) {
  if (realtimeSetup) {
    console.log('[Realtime] Already initialized')
    return
  }

  const { setupRealtimeSubscriptions } = useSupabaseRealtime()
  await setupRealtimeSubscriptions(organizationId)
  realtimeSetup = true
}

export async function disconnectRealtime() {
  const { cleanupSubscriptions } = useSupabaseRealtime()
  await cleanupSubscriptions()
  realtimeSetup = false
}
