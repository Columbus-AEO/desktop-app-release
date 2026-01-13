<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useUiStore } from '@/stores/ui'
import { useProductsStore } from '@/stores/products'
import { useRegionsStore } from '@/stores/regions'
import { usePlatformsStore } from '@/stores/platforms'
import { useScanStore } from '@/stores/scan'
import { useTauriCommands } from '@/composables/useTauriCommands'
import AppHeader from '@/components/common/AppHeader.vue'
import ProductSelector from '@/components/settings/ProductSelector.vue'
import AuthStatusGrid from '@/components/auth/AuthStatusGrid.vue'
import AutoRunSettings from '@/components/settings/AutoRunSettings.vue'
import ScheduleInfo from '@/components/settings/ScheduleInfo.vue'
import BaseButton from '@/components/common/BaseButton.vue'
import { DASHBOARD_URL, TUTORIAL_URL } from '@/utils/constants'

const authStore = useAuthStore()
const uiStore = useUiStore()
const productsStore = useProductsStore()
const regionsStore = useRegionsStore()
const platformsStore = usePlatformsStore()
const scanStore = useScanStore()
const commands = useTauriCommands()

const hasProduct = computed(() => !!productsStore.selectedProductId)
const hasAuth = computed(() => regionsStore.hasAnyAuth())

const scanInfoText = computed(() => {
  if (!hasProduct.value && !hasAuth.value) {
    return 'Select a product and authenticate platforms'
  }
  if (!hasProduct.value) {
    return 'Select a product to start scanning'
  }
  if (!hasAuth.value) {
    return 'Authenticate at least one platform to scan'
  }

  const product = productsStore.selectedProduct
  const authPlatformCount = platformsStore.platformIds.filter(p =>
    regionsStore.configuredRegions.some(r => regionsStore.isPlatformAuthForRegion(r, p))
  ).length

  const samples = productsStore.productConfig?.samples_per_prompt || 1
  const maxTests = productsStore.promptCount * samples * authPlatformCount
  const availableTests = productsStore.availableTests
  const { dailyUsage } = productsStore

  if (dailyUsage.isUnlimited) {
    return `Ready to scan ${product?.name} (up to ${maxTests} tests: ${productsStore.promptCount} prompts × ${authPlatformCount} platforms${samples > 1 ? ` × ${samples} samples` : ''})`
  }

  if (productsStore.promptCount > 0) {
    if (availableTests <= 0) {
      if (dailyUsage.pendingEvaluations > 0) {
        return `Please wait - ${dailyUsage.pendingEvaluations} tests pending evaluation`
      }
      return `Daily limit reached (${dailyUsage.current}/${dailyUsage.limit})`
    }

    if (maxTests > availableTests) {
      return `Will run ${availableTests} of ${maxTests} tests (${dailyUsage.current}/${dailyUsage.limit} used)`
    }

    const pending = dailyUsage.pendingEvaluations > 0
      ? ` (${dailyUsage.pendingEvaluations} pending)`
      : ''
    return `Up to ${maxTests} tests (${productsStore.promptCount} prompts × ${authPlatformCount} platforms)${pending} (${availableTests} available)`
  }

  return `Ready to scan ${product?.name}`
})

const canScan = computed(() => {
  // Only disable when quota is exhausted (unlimited plans can always scan)
  return productsStore.dailyUsage.isUnlimited || productsStore.availableTests > 0
})

function openDashboard() {
  commands.openUrlInBrowser(DASHBOARD_URL)
}

function openTutorial() {
  commands.openUrlInBrowser(TUTORIAL_URL)
}

function openSettings() {
  uiStore.showModal('settings')
}

function openRegionAuth() {
  uiStore.showView('region-auth')
}

function openKeywordDiscovery() {
  if (!productsStore.selectedProductId) {
    uiStore.showMessage('Please select a product first.', 'No Product Selected', 'warning')
    return
  }
  uiStore.showModal('keywordDiscovery')
}

async function handleLogout() {
  await authStore.logout()
}

async function handleStartScan() {
  await scanStore.startScan()
}
</script>

<template>
  <div class="h-full flex flex-col bg-gray-50">
    <!-- Header -->
    <AppHeader />

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      <!-- User Info & Actions -->
      <div class="flex items-center justify-between">
        <span class="text-sm text-gray-600 truncate">{{ authStore.userEmail }}</span>
        <div class="flex items-center gap-2">
          <button
            @click="openTutorial"
            class="text-sm text-gray-500 hover:text-gray-700"
          >
            Tutorial
          </button>
          <button
            @click="openSettings"
            class="text-sm text-gray-500 hover:text-gray-700"
          >
            Settings
          </button>
          <button
            @click="handleLogout"
            class="text-sm text-red-500 hover:text-red-600"
          >
            Logout
          </button>
        </div>
      </div>

      <!-- Product Selector -->
      <ProductSelector />

      <!-- Auth Status Grid -->
      <div class="bg-white rounded-lg p-4 shadow-sm">
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-sm font-medium text-gray-700">Authentication Status</h3>
          <button
            @click="openRegionAuth"
            class="text-xs text-brand hover:text-brand-dark"
          >
            Manage
          </button>
        </div>
        <AuthStatusGrid />
      </div>

      <!-- Auto-Run Settings -->
      <AutoRunSettings />

      <!-- Schedule Info -->
      <ScheduleInfo />
    </div>

    <!-- Bottom Actions -->
    <div class="p-4 border-t border-gray-200 bg-white space-y-3">
      <!-- Scan Info -->
      <p class="text-xs text-gray-500 text-center">
        {{ scanInfoText }}
      </p>

      <!-- Buttons -->
      <div class="flex gap-2">
        <BaseButton
          variant="primary"
          :disabled="!canScan || scanStore.isScanning || scanStore.isStarting"
          :loading="scanStore.isStarting"
          full-width
          @click="handleStartScan"
        >
          {{ scanStore.isStarting ? 'Starting...' : 'Run Scan' }}
        </BaseButton>
        <BaseButton
          variant="secondary"
          :disabled="!hasProduct"
          @click="openKeywordDiscovery"
        >
          Find Keywords
        </BaseButton>
      </div>

      <!-- Dashboard Link -->
      <button
        @click="openDashboard"
        class="w-full text-center text-sm text-brand hover:text-brand-dark"
      >
        View Dashboard →
      </button>
    </div>
  </div>
</template>
