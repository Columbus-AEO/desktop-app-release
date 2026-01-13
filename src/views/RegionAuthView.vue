<script setup lang="ts">
import { ref } from 'vue'
import { useUiStore } from '@/stores/ui'
import { useRegionsStore } from '@/stores/regions'
import { usePlatformsStore } from '@/stores/platforms'
import { useTauriCommands } from '@/composables/useTauriCommands'
import RegionTabs from '@/components/auth/RegionTabs.vue'
import PlatformAuthCard from '@/components/auth/PlatformAuthCard.vue'
import BaseButton from '@/components/common/BaseButton.vue'
import BaseInput from '@/components/common/BaseInput.vue'

const uiStore = useUiStore()
const regionsStore = useRegionsStore()
const platformsStore = usePlatformsStore()
const commands = useTauriCommands()

const magicLinkUrl = ref('')

async function handleBack() {
  await commands.setOnboardingCompleted(true)
  uiStore.showView('main')
}

function showAddRegion() {
  uiStore.showModal('addRegion')
}

async function handleOpenMagicLink() {
  const url = magicLinkUrl.value.trim()
  if (!url) {
    uiStore.showMessage('Please paste a URL first', 'URL Required', 'warning')
    return
  }

  try {
    await commands.openMagicLink(regionsStore.selectedAuthRegion, url)
    magicLinkUrl.value = ''
  } catch (error) {
    uiStore.showMessage('Failed to open URL: ' + error, 'Error', 'error')
  }
}

async function handleAuthPlatform(platform: string) {
  try {
    await commands.openCountryLogin(regionsStore.selectedAuthRegion, platform, true)
  } catch (error) {
    uiStore.showMessage('Failed to open authentication: ' + error, 'Error', 'error')
  }
}

async function handleToggleAuth(platform: string) {
  const currentStatus = regionsStore.isPlatformAuthForRegion(
    regionsStore.selectedAuthRegion,
    platform
  )
  await regionsStore.setAuthStatus(
    regionsStore.selectedAuthRegion,
    platform,
    !currentStatus
  )
}
</script>

<template>
  <div class="h-full flex flex-col bg-gray-50">
    <!-- Header -->
    <div class="p-4 bg-white border-b border-gray-200">
      <h2 class="text-lg font-semibold text-gray-900 text-center">Authenticate Platforms</h2>
      <p class="text-xs text-gray-500 text-center mt-1">
        Log in to each AI platform to enable scanning
      </p>
    </div>

    <!-- Region Tabs -->
    <RegionTabs />

    <!-- Add Region Button -->
    <div class="px-4 py-2">
      <button
        @click="showAddRegion"
        class="w-full py-2 border-2 border-dashed border-gray-300 rounded-lg text-sm text-gray-500 hover:border-brand hover:text-brand transition-colors"
      >
        + Add Region
      </button>
    </div>

    <!-- Platform Cards -->
    <div class="flex-1 overflow-y-auto p-4 space-y-2">
      <PlatformAuthCard
        v-for="platform in platformsStore.platformIds"
        :key="platform"
        :platform-id="platform"
        :region="regionsStore.selectedAuthRegion"
        @login="handleAuthPlatform"
        @toggle="handleToggleAuth"
      />
    </div>

    <!-- Magic Link Section -->
    <div class="p-4 bg-white border-t border-gray-200">
      <p class="text-xs text-gray-500 mb-2">
        Need to open a magic link or 2FA URL?
      </p>
      <div class="flex gap-2">
        <BaseInput
          v-model="magicLinkUrl"
          placeholder="Paste URL here..."
          class="flex-1"
        />
        <BaseButton
          variant="secondary"
          @click="handleOpenMagicLink"
        >
          Open
        </BaseButton>
      </div>
    </div>

    <!-- Done Button -->
    <div class="p-4 bg-white border-t border-gray-200">
      <BaseButton
        variant="primary"
        full-width
        @click="handleBack"
        class="py-3 text-base font-semibold"
      >
        Done
      </BaseButton>
      <p class="text-xs text-gray-400 text-center mt-2">
        You can always come back to manage authentication later
      </p>
    </div>
  </div>
</template>
