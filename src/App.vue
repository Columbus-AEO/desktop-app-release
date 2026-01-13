<script setup lang="ts">
import { onMounted } from 'vue'
import { useUiStore } from '@/stores/ui'
import { useAuthStore } from '@/stores/auth'
import { usePlatformsStore } from '@/stores/platforms'
import { useInstancesStore } from '@/stores/instances'
import { useTauriEvents } from '@/composables/useTauriEvents'
import LoadingScreen from '@/components/common/LoadingScreen.vue'
import LoginView from '@/views/LoginView.vue'
import OnboardingView from '@/views/OnboardingView.vue'
import RegionAuthView from '@/views/RegionAuthView.vue'
import MainView from '@/views/MainView.vue'
import ScanningView from '@/views/ScanningView.vue'
import CompleteView from '@/views/CompleteView.vue'
import SettingsModal from '@/components/modals/SettingsModal.vue'
import AddRegionModal from '@/components/modals/AddRegionModal.vue'
import AuthRequiredModal from '@/components/modals/AuthRequiredModal.vue'
import InstanceRenameModal from '@/components/modals/InstanceRenameModal.vue'
import MessageModal from '@/components/modals/MessageModal.vue'
import KeywordDiscoveryModal from '@/components/modals/KeywordDiscoveryModal.vue'

const uiStore = useUiStore()
const authStore = useAuthStore()
const platformsStore = usePlatformsStore()
const instancesStore = useInstancesStore()
const { setupEventListeners } = useTauriEvents()

onMounted(async () => {
  try {
    // Set up event listeners first
    await setupEventListeners()

    // Load instances and platforms in parallel
    await Promise.all([
      instancesStore.loadInstances(),
      platformsStore.loadPlatforms(),
    ])

    // Check auth status (this will route to appropriate view)
    await authStore.checkAuthStatus()

    // Check for updates in background
    uiStore.checkForUpdates()
  } catch (error) {
    console.error('App initialization failed:', error)
  } finally {
    uiStore.setInitializing(false)
  }
})
</script>

<template>
  <div class="h-screen w-screen overflow-hidden bg-gray-50">
    <!-- Loading Screen -->
    <LoadingScreen v-if="uiStore.isInitializing" />

    <!-- Main Content -->
    <div v-else class="h-full">
      <!-- Views -->
      <LoginView v-if="uiStore.currentView === 'login'" />
      <OnboardingView v-else-if="uiStore.currentView === 'onboarding'" />
      <RegionAuthView v-else-if="uiStore.currentView === 'region-auth'" />
      <MainView v-else-if="uiStore.currentView === 'main'" />
      <ScanningView v-else-if="uiStore.currentView === 'scanning'" />
      <CompleteView v-else-if="uiStore.currentView === 'complete'" />
    </div>

    <!-- Modals -->
    <SettingsModal v-if="uiStore.modals.settings" />
    <AddRegionModal v-if="uiStore.modals.addRegion" />
    <AuthRequiredModal v-if="uiStore.modals.authRequired" />
    <InstanceRenameModal v-if="uiStore.modals.instanceRename.visible" />
    <MessageModal v-if="uiStore.modals.message.visible" />
    <KeywordDiscoveryModal v-if="uiStore.modals.keywordDiscovery" />

    <!-- Update Banner -->
    <div
      v-if="uiStore.updateAvailable"
      class="fixed top-0 left-0 right-0 bg-brand text-white px-4 py-2 flex items-center justify-between z-50"
    >
      <div class="flex items-center gap-2">
        <span>🔄</span>
        <span>Version {{ uiStore.updateAvailable.version }} is available!</span>
      </div>
      <div class="flex items-center gap-2">
        <button
          @click="uiStore.installUpdate"
          :disabled="uiStore.isUpdating"
          class="bg-white text-brand px-3 py-1 rounded text-sm font-medium hover:bg-gray-100 disabled:opacity-50"
        >
          {{ uiStore.updateButtonText }}
        </button>
        <button
          @click="uiStore.dismissUpdate"
          class="text-white/80 hover:text-white text-xl leading-none"
        >
          &times;
        </button>
      </div>
    </div>
  </div>
</template>
