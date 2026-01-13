<script setup lang="ts">
import { computed } from 'vue'
import { usePlatformsStore } from '@/stores/platforms'
import ProgressBar from '@/components/common/ProgressBar.vue'
import type { PlatformState } from '@/types'

const props = defineProps<{
  platformId: string
  state?: PlatformState
}>()

const platformsStore = usePlatformsStore()

const platformName = computed(() =>
  platformsStore.getPlatformName(props.platformId)
)

const platformLogo = computed(() =>
  platformsStore.getPlatformLogo(props.platformId)
)

const status = computed(() => props.state?.status || 'pending')

const progress = computed(() => {
  if (!props.state) return 0
  const { submitted, collected, total } = props.state
  if (total === 0) return 0
  return Math.round(((submitted + collected) / (total * 2)) * 100)
})

const countText = computed(() => {
  if (!props.state) return '0/0'
  return `${props.state.collected}/${props.state.total}`
})

const statusClass = computed(() => {
  switch (status.value) {
    case 'submitting':
    case 'collecting':
      return 'text-blue-600'
    case 'complete':
      return 'text-green-600'
    case 'error':
      return 'text-red-600'
    default:
      return 'text-gray-400'
  }
})
</script>

<template>
  <div class="bg-white rounded-lg p-3 shadow-sm">
    <div class="flex items-center gap-3 mb-2">
      <!-- Platform Icon -->
      <div
        v-if="platformLogo"
        class="w-6 h-6 rounded overflow-hidden bg-gray-100"
      >
        <img :src="platformLogo" :alt="platformName" class="w-full h-full object-cover" />
      </div>
      <div
        v-else
        class="w-6 h-6 rounded"
        :class="`platform-${platformId}`"
      />

      <!-- Name -->
      <span class="flex-1 text-sm font-medium text-gray-700">{{ platformName }}</span>

      <!-- Status -->
      <span class="text-xs font-medium" :class="statusClass">
        {{ status }}
      </span>
    </div>

    <!-- Progress -->
    <div class="flex items-center gap-2">
      <div class="flex-1">
        <ProgressBar
          :value="progress"
          size="sm"
          :color="status === 'complete' ? 'green' : 'brand'"
        />
      </div>
      <span class="text-xs text-gray-500 w-12 text-right">{{ countText }}</span>
    </div>
  </div>
</template>
