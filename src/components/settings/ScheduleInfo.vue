<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useProductsStore } from '@/stores/products'

const productsStore = useProductsStore()

const nextScanTime = computed(() => {
  const info = productsStore.scheduleInfo
  if (info?.next_scan_hour !== null && info?.next_scan_hour !== undefined) {
    return `${info.next_scan_hour.toString().padStart(2, '0')}:00`
  }
  return '--'
})

const scansCompleted = computed(() =>
  productsStore.scheduleInfo?.scans_completed_today || 0
)

const scansTotal = computed(() =>
  productsStore.scheduleInfo?.scans_total_today || 1
)

const isAutoRunEnabled = computed(() =>
  productsStore.productConfig?.auto_run_enabled ?? true
)

onMounted(() => {
  productsStore.loadScheduleInfo()
})
</script>

<template>
  <div
    v-if="productsStore.selectedProductId"
    class="bg-white rounded-lg p-4 shadow-sm"
    :class="{ 'opacity-50': !isAutoRunEnabled }"
  >
    <h3 class="text-sm font-medium text-gray-700 mb-3">Schedule</h3>

    <div class="grid grid-cols-3 gap-4 text-center">
      <div>
        <p class="text-lg font-semibold text-brand">{{ nextScanTime }}</p>
        <p class="text-xs text-gray-500">Next scan</p>
      </div>
      <div>
        <p class="text-lg font-semibold text-gray-900">{{ scansCompleted }}</p>
        <p class="text-xs text-gray-500">Completed</p>
      </div>
      <div>
        <p class="text-lg font-semibold text-gray-900">{{ scansTotal }}</p>
        <p class="text-xs text-gray-500">Total today</p>
      </div>
    </div>
  </div>
</template>
