<script setup lang="ts">
interface Props {
  value: number
  max?: number
  showLabel?: boolean
  size?: 'sm' | 'md' | 'lg'
  color?: 'brand' | 'green' | 'blue'
}

const props = withDefaults(defineProps<Props>(), {
  max: 100,
  showLabel: false,
  size: 'md',
  color: 'brand'
})

const percentage = computed(() => Math.min(100, Math.max(0, (props.value / props.max) * 100)))

const heightClasses = {
  sm: 'h-1',
  md: 'h-2',
  lg: 'h-3'
}

const colorClasses = {
  brand: 'bg-brand',
  green: 'bg-green-500',
  blue: 'bg-blue-500'
}

import { computed } from 'vue'
</script>

<template>
  <div class="w-full">
    <div v-if="showLabel" class="flex justify-between mb-1">
      <span class="text-xs text-gray-600">Progress</span>
      <span class="text-xs font-medium text-gray-800">{{ Math.round(percentage) }}%</span>
    </div>
    <div class="w-full bg-gray-200 rounded-full overflow-hidden" :class="heightClasses[size]">
      <div
        class="h-full rounded-full transition-all duration-300 ease-out"
        :class="colorClasses[color]"
        :style="{ width: `${percentage}%` }"
      />
    </div>
  </div>
</template>
