<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '@/stores/ui'
import { useRegionsStore } from '@/stores/regions'
import BaseModal from '@/components/common/BaseModal.vue'

const uiStore = useUiStore()
const regionsStore = useRegionsStore()

const unconfiguredRegions = computed(() =>
  regionsStore.allRegions.filter(r => !regionsStore.configuredRegions.includes(r.code))
)

function close() {
  uiStore.hideModal('addRegion')
}

async function selectRegion(code: string) {
  await regionsStore.addRegion(code)
  regionsStore.selectRegion(code)
  close()
}
</script>

<template>
  <BaseModal title="Add Region" @close="close">
    <div v-if="unconfiguredRegions.length === 0" class="text-center py-4 text-gray-500">
      All available regions have been added.
    </div>

    <div v-else class="space-y-2">
      <button
        v-for="region in unconfiguredRegions"
        :key="region.code"
        @click="selectRegion(region.code)"
        class="w-full flex items-center justify-between p-3 bg-gray-50 hover:bg-gray-100 rounded-lg transition-colors"
      >
        <span class="flex items-center gap-2">
          <span class="text-lg">{{ region.flag_emoji }}</span>
          <span class="text-sm text-gray-700">{{ region.name }}</span>
        </span>
        <span class="text-xs text-gray-400 uppercase">{{ region.code }}</span>
      </button>
    </div>
  </BaseModal>
</template>
