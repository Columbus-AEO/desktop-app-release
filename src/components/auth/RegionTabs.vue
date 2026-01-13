<script setup lang="ts">
import { useRegionsStore } from '@/stores/regions'
import { usePlatformsStore } from '@/stores/platforms'

const regionsStore = useRegionsStore()
const platformsStore = usePlatformsStore()

function selectRegion(code: string) {
  regionsStore.selectRegion(code)
}

async function removeRegion(code: string) {
  if (code === 'local') return

  if (confirm(`Remove ${code.toUpperCase()} region? You'll need to re-authenticate platforms if you add it back.`)) {
    await regionsStore.removeRegion(code)

    // Select another region if we removed the selected one
    if (regionsStore.selectedAuthRegion === code) {
      regionsStore.selectRegion(regionsStore.configuredRegions[0] || 'local')
    }
  }
}
</script>

<template>
  <div class="flex overflow-x-auto bg-white px-4 py-2 gap-2 border-b border-gray-200">
    <button
      v-for="region in regionsStore.configuredRegions"
      :key="region"
      @click="selectRegion(region)"
      class="flex-shrink-0 flex items-center gap-1 px-3 py-1.5 rounded-lg text-sm transition-colors"
      :class="[
        region === regionsStore.selectedAuthRegion
          ? 'bg-brand text-white'
          : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
      ]"
    >
      <span>{{ regionsStore.getRegionFlag(region) }}</span>
      <span>{{ region === 'local' ? 'Local' : region.toUpperCase() }}</span>
      <span class="text-xs opacity-70">
        {{ regionsStore.getAuthCountForRegion(region) }}/{{ platformsStore.platformIds.length }}
      </span>

      <!-- Remove button (not for local) -->
      <button
        v-if="region !== 'local'"
        @click.stop="removeRegion(region)"
        class="ml-1 text-current opacity-50 hover:opacity-100"
      >
        &times;
      </button>
    </button>
  </div>
</template>
