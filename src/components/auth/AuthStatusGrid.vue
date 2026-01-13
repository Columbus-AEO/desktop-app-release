<script setup lang="ts">
import { useRegionsStore } from '@/stores/regions'
import { usePlatformsStore } from '@/stores/platforms'
import { capitalizeFirst } from '@/utils/constants'

const regionsStore = useRegionsStore()
const platformsStore = usePlatformsStore()
</script>

<template>
  <div class="flex flex-wrap gap-2">
    <div
      v-for="region in regionsStore.configuredRegions"
      :key="region"
      class="flex items-center gap-2 px-2 py-1 bg-gray-100 rounded-lg"
    >
      <!-- Region name -->
      <span class="text-xs text-gray-600">
        {{ regionsStore.getRegionFlag(region) }}
        {{ region === 'local' ? 'Local' : region.toUpperCase() }}
      </span>

      <!-- Platform dots -->
      <div class="flex items-center gap-0.5">
        <span
          v-for="platform in platformsStore.platformIds"
          :key="platform"
          class="w-2 h-2 rounded-full"
          :class="[
            regionsStore.isPlatformAuthForRegion(region, platform)
              ? `platform-${platform}`
              : 'bg-gray-300'
          ]"
          :title="`${capitalizeFirst(platform)}: ${regionsStore.isPlatformAuthForRegion(region, platform) ? 'Authenticated' : 'Not authenticated'}`"
        />
      </div>
    </div>
  </div>
</template>
