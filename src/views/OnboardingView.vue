<script setup lang="ts">
import { ref, computed } from 'vue'
import { useUiStore } from '@/stores/ui'
import { useRegionsStore } from '@/stores/regions'
import { useTauriCommands } from '@/composables/useTauriCommands'
import BaseButton from '@/components/common/BaseButton.vue'
import { TUTORIAL_URL } from '@/utils/constants'

const uiStore = useUiStore()
const regionsStore = useRegionsStore()
const commands = useTauriCommands()

const selectedRegions = ref<Set<string>>(new Set(['local']))

const allRegions = computed(() => regionsStore.allRegions)

function toggleRegion(code: string) {
  if (selectedRegions.value.has(code)) {
    // Can't remove if it's the only one
    if (selectedRegions.value.size > 1) {
      selectedRegions.value.delete(code)
    }
  } else {
    selectedRegions.value.add(code)
  }
}

async function handleContinue() {
  if (selectedRegions.value.size === 0) {
    uiStore.showMessage('Please select at least one region', 'Region Required', 'warning')
    return
  }

  // Add selected regions
  for (const region of selectedRegions.value) {
    if (!regionsStore.configuredRegions.includes(region)) {
      await regionsStore.addRegion(region)
    }
  }

  // Go to region auth view
  regionsStore.selectRegion(regionsStore.configuredRegions[0] || 'local')
  uiStore.showView('region-auth')
}

function openTutorial() {
  commands.openUrlInBrowser(TUTORIAL_URL)
}
</script>

<template>
  <div class="h-full flex flex-col bg-gradient-to-b from-gray-50 to-gray-100 p-6">
    <!-- Header -->
    <div class="text-center mb-6">
      <img
        src="@/assets/icon-128.png"
        alt="Columbus"
        class="w-12 h-12 mx-auto mb-4"
      />
      <h1 class="text-xl font-bold text-gray-900">Welcome to Columbus</h1>
      <p class="text-sm text-gray-500 mt-1">AI Brand Monitor Desktop</p>
    </div>

    <!-- Tutorial Video -->
    <div class="bg-white rounded-lg p-4 shadow-sm mb-4">
      <h3 class="text-sm font-medium text-gray-700 mb-2">Getting Started</h3>
      <p class="text-xs text-gray-500 mb-3">
        Watch our quick tutorial to learn how to use Columbus
      </p>
      <button
        @click="openTutorial"
        class="w-full flex items-center justify-center gap-2 py-3 bg-red-50 text-red-600 rounded-lg hover:bg-red-100 transition-colors"
      >
        <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
          <path d="M19.615 3.184c-3.604-.246-11.631-.245-15.23 0-3.897.266-4.356 2.62-4.385 8.816.029 6.185.484 8.549 4.385 8.816 3.6.245 11.626.246 15.23 0 3.897-.266 4.356-2.62 4.385-8.816-.029-6.185-.484-8.549-4.385-8.816zm-10.615 12.816v-8l8 3.993-8 4.007z"/>
        </svg>
        Watch Tutorial
      </button>
    </div>

    <!-- Region Selection -->
    <div class="flex-1 overflow-y-auto">
      <h3 class="text-sm font-medium text-gray-700 mb-2">Select Regions</h3>
      <p class="text-xs text-gray-500 mb-3">
        Choose which regions you want to scan from
      </p>
      <div class="grid grid-cols-2 gap-2">
        <label
          v-for="region in allRegions"
          :key="region.code"
          class="relative flex flex-col items-center p-4 rounded-xl cursor-pointer transition-all duration-200"
          :class="selectedRegions.has(region.code)
            ? 'bg-brand/10 border-2 border-brand shadow-md'
            : 'bg-white border-2 border-gray-100 hover:border-gray-200 hover:shadow-sm'"
        >
          <input
            type="checkbox"
            :checked="selectedRegions.has(region.code)"
            @change="toggleRegion(region.code)"
            class="sr-only"
          />
          <!-- Checkmark badge -->
          <div
            v-if="selectedRegions.has(region.code)"
            class="absolute top-2 right-2 w-5 h-5 bg-brand rounded-full flex items-center justify-center shadow-sm"
          >
            <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <!-- Flag -->
          <span class="text-3xl mb-2">{{ region.flag_emoji }}</span>
          <!-- Name -->
          <span
            class="text-xs font-medium text-center leading-tight"
            :class="selectedRegions.has(region.code) ? 'text-brand-dark' : 'text-gray-600'"
          >
            {{ region.name }}
          </span>
        </label>
      </div>
    </div>

    <!-- Continue Button -->
    <div class="mt-4">
      <BaseButton
        variant="primary"
        full-width
        @click="handleContinue"
      >
        Continue to Authentication
      </BaseButton>
    </div>
  </div>
</template>
