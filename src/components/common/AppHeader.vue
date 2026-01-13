<script setup lang="ts">
import { useUiStore } from '@/stores/ui'
import InstanceSwitcher from './InstanceSwitcher.vue'

const uiStore = useUiStore()
</script>

<template>
  <div class="bg-white border-b border-gray-200 p-4">
    <div class="flex items-center justify-between">
      <!-- Logo and Name -->
      <div class="flex items-center gap-2">
        <img
          src="@/assets/icon-32.png"
          alt="Columbus"
          class="w-6 h-6"
        />
        <span class="font-semibold text-gray-900">Columbus</span>
      </div>

      <!-- Instance Switcher -->
      <InstanceSwitcher />
    </div>

    <!-- Version -->
    <div v-if="uiStore.appVersion" class="mt-1 text-xs text-gray-400">
      Desktop v{{ uiStore.appVersion }}
    </div>
  </div>
</template>
