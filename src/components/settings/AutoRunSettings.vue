<script setup lang="ts">
import { computed } from 'vue'
import { useProductsStore } from '@/stores/products'
import ToggleSwitch from '@/components/common/ToggleSwitch.vue'

const productsStore = useProductsStore()

const autoRunEnabled = computed({
  get: () => productsStore.productConfig?.auto_run_enabled ?? true,
  set: (value) => {
    productsStore.updateConfig({ auto_run_enabled: value })
    productsStore.saveProductConfig()
  }
})

const scansPerDay = computed({
  get: () => productsStore.productConfig?.scans_per_day || 1,
  set: (value) => {
    productsStore.updateConfig({ scans_per_day: value })
    productsStore.saveProductConfig()
  }
})

const timeWindowStart = computed({
  get: () => productsStore.productConfig?.time_window_start ?? 9,
  set: (value) => {
    productsStore.updateConfig({ time_window_start: value })
    productsStore.saveProductConfig()
  }
})

const timeWindowEnd = computed({
  get: () => productsStore.productConfig?.time_window_end ?? 17,
  set: (value) => {
    productsStore.updateConfig({ time_window_end: value })
    productsStore.saveProductConfig()
  }
})

const samplesPerPrompt = computed({
  get: () => productsStore.productConfig?.samples_per_prompt || 1,
  set: (value) => {
    productsStore.updateConfig({ samples_per_prompt: value })
    productsStore.saveProductConfig()
  }
})

const showSamplesWarning = computed(() => samplesPerPrompt.value > 3)

const autostartEnabled = computed({
  get: () => productsStore.autostartEnabled,
  set: (value) => productsStore.setAutostartEnabled(value)
})

const hours = Array.from({ length: 24 }, (_, i) => ({
  value: i,
  label: `${i.toString().padStart(2, '0')}:00`
}))
</script>

<template>
  <div class="bg-white rounded-lg p-4 shadow-sm space-y-4">
    <h3 class="text-sm font-medium text-gray-700">Settings</h3>

    <!-- Samples per prompt -->
    <div class="flex items-center justify-between">
      <label class="text-sm text-gray-600">Samples per prompt</label>
      <select
        :value="samplesPerPrompt"
        @change="samplesPerPrompt = parseInt(($event.target as HTMLSelectElement).value)"
        class="px-2 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-brand"
      >
        <option v-for="n in 5" :key="n" :value="n">{{ n }}</option>
      </select>
    </div>
    <p
      v-if="showSamplesWarning"
      class="text-xs text-amber-600 bg-amber-50 px-2 py-1 rounded"
    >
      Higher values use more of your daily quota
    </p>

    <!-- Auto-run toggle -->
    <div class="flex items-center justify-between">
      <span class="text-sm text-gray-600">Auto-run scans</span>
      <ToggleSwitch v-model="autoRunEnabled" />
    </div>

    <!-- Scans per day (conditional) -->
    <div
      v-if="autoRunEnabled"
      class="flex items-center justify-between"
      :class="{ 'opacity-50': !autoRunEnabled }"
    >
      <label class="text-sm text-gray-600">Scans per day</label>
      <select
        :value="scansPerDay"
        @change="scansPerDay = parseInt(($event.target as HTMLSelectElement).value)"
        :disabled="!autoRunEnabled"
        class="px-2 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-brand disabled:opacity-50"
      >
        <option v-for="n in 4" :key="n" :value="n">{{ n }}</option>
      </select>
    </div>

    <!-- Time window (conditional) -->
    <div
      v-if="autoRunEnabled"
      class="flex items-center justify-between gap-2"
      :class="{ 'opacity-50': !autoRunEnabled }"
    >
      <label class="text-sm text-gray-600">Time window</label>
      <div class="flex items-center gap-1">
        <select
          :value="timeWindowStart"
          @change="timeWindowStart = parseInt(($event.target as HTMLSelectElement).value)"
          :disabled="!autoRunEnabled"
          class="px-2 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-brand disabled:opacity-50"
        >
          <option v-for="hour in hours" :key="hour.value" :value="hour.value">
            {{ hour.label }}
          </option>
        </select>
        <span class="text-gray-400">-</span>
        <select
          :value="timeWindowEnd"
          @change="timeWindowEnd = parseInt(($event.target as HTMLSelectElement).value)"
          :disabled="!autoRunEnabled"
          class="px-2 py-1 border border-gray-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-brand disabled:opacity-50"
        >
          <option v-for="hour in hours" :key="hour.value" :value="hour.value">
            {{ hour.label }}
          </option>
        </select>
      </div>
    </div>

    <!-- Autostart toggle -->
    <div class="flex items-center justify-between pt-2 border-t border-gray-100">
      <span class="text-sm text-gray-600">Start with system</span>
      <ToggleSwitch v-model="autostartEnabled" />
    </div>
  </div>
</template>
