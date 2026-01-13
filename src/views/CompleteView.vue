<script setup lang="ts">
import { useScanStore } from '@/stores/scan'
import { useUiStore } from '@/stores/ui'
import { useTauriCommands } from '@/composables/useTauriCommands'
import BaseButton from '@/components/common/BaseButton.vue'
import ScanCompleteStats from '@/components/scan/ScanCompleteStats.vue'
import { DASHBOARD_URL } from '@/utils/constants'

const scanStore = useScanStore()
const uiStore = useUiStore()
const commands = useTauriCommands()

function viewResults() {
  commands.openUrlInBrowser(DASHBOARD_URL)
}

function newScan() {
  uiStore.showView('main')
}
</script>

<template>
  <div class="h-full flex flex-col items-center justify-center bg-gray-50 p-6">
    <!-- Success Icon -->
    <div class="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mb-6">
      <svg class="w-8 h-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
      </svg>
    </div>

    <!-- Title -->
    <h2 class="text-xl font-semibold text-gray-900 mb-2">Scan Complete!</h2>
    <p class="text-sm text-gray-500 mb-6">Your visibility data has been collected</p>

    <!-- Stats -->
    <ScanCompleteStats v-if="scanStore.lastResult" :result="scanStore.lastResult" />

    <!-- Actions -->
    <div class="w-full max-w-xs space-y-3 mt-6">
      <BaseButton
        variant="primary"
        full-width
        @click="viewResults"
      >
        View Results
      </BaseButton>
      <BaseButton
        variant="secondary"
        full-width
        @click="newScan"
      >
        Run Another Scan
      </BaseButton>
    </div>
  </div>
</template>
