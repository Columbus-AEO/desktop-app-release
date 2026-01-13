<script setup lang="ts">
import type { ScanComplete } from '@/types'

defineProps<{
  result: ScanComplete
}>()
</script>

<template>
  <div class="w-full grid grid-cols-2 gap-3">
    <div class="bg-white rounded-lg p-3 shadow-sm text-center">
      <p class="text-2xl font-bold text-gray-900">{{ result.total_prompts }}</p>
      <p class="text-xs text-gray-500">Prompts Tested</p>
    </div>

    <div class="bg-white rounded-lg p-3 shadow-sm text-center">
      <p class="text-2xl font-bold text-brand">{{ Math.round(result.mention_rate) }}%</p>
      <p class="text-xs text-gray-500">Mention Rate</p>
    </div>

    <div class="bg-white rounded-lg p-3 shadow-sm text-center">
      <p class="text-2xl font-bold text-gray-900">{{ result.successful_prompts }}</p>
      <p class="text-xs text-gray-500">Successful</p>
    </div>

    <div class="bg-white rounded-lg p-3 shadow-sm text-center">
      <p class="text-2xl font-bold text-green-600">{{ Math.round(result.citation_rate) }}%</p>
      <p class="text-xs text-gray-500">Citation Rate</p>
    </div>
  </div>
</template>
