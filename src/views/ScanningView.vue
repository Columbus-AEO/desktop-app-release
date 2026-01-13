<script setup lang="ts">
import { useScanStore } from '@/stores/scan'
import { usePlatformsStore } from '@/stores/platforms'
import ProgressBar from '@/components/common/ProgressBar.vue'
import BaseButton from '@/components/common/BaseButton.vue'
import PlatformProgressItem from '@/components/scan/PlatformProgressItem.vue'

const scanStore = useScanStore()
const platformsStore = usePlatformsStore()

async function handleCancel() {
  await scanStore.cancelScan()
}
</script>

<template>
  <div class="h-full flex flex-col bg-gray-50 p-4">
    <!-- Header -->
    <div class="text-center mb-6">
      <h2 class="text-lg font-semibold text-gray-900">Scanning in Progress</h2>
      <p class="text-sm text-gray-500 mt-1">{{ scanStore.phaseDisplayText }}</p>
    </div>

    <!-- Overall Progress -->
    <div class="bg-white rounded-lg p-4 shadow-sm mb-4">
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm font-medium text-gray-700">Overall Progress</span>
        <span class="text-sm font-bold text-brand">{{ scanStore.progress }}%</span>
      </div>
      <ProgressBar :value="scanStore.progress" size="lg" />
    </div>

    <!-- Countdown -->
    <div
      v-if="scanStore.countdownSeconds !== null"
      class="bg-amber-50 border border-amber-200 rounded-lg p-3 mb-4 text-center"
    >
      <p class="text-sm text-amber-800">
        Waiting for responses...
        <span class="font-bold">{{ scanStore.countdownSeconds }}s</span>
      </p>
    </div>

    <!-- Platform Progress Grid -->
    <div class="flex-1 overflow-y-auto space-y-2">
      <PlatformProgressItem
        v-for="platformId in platformsStore.platformIds"
        :key="platformId"
        :platform-id="platformId"
        :state="scanStore.platforms[platformId]"
      />
    </div>

    <!-- Cancel Button -->
    <div class="mt-4">
      <BaseButton
        variant="secondary"
        full-width
        @click="handleCancel"
      >
        Cancel Scan
      </BaseButton>
    </div>
  </div>
</template>
