<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '@/stores/ui'
import BaseModal from '@/components/common/BaseModal.vue'
import BaseButton from '@/components/common/BaseButton.vue'

const uiStore = useUiStore()

const missingAuth = computed(() => uiStore.authRequiredData)
const hasItems = computed(() => missingAuth.value.length > 0)

function close() {
  uiStore.hideModal('authRequired')
}

function goToAuth() {
  close()
  uiStore.showView('region-auth')
}
</script>

<template>
  <BaseModal title="Authentication Required" @close="close">
    <div class="space-y-4">
      <p v-if="hasItems" class="text-sm text-gray-600">
        The following regions need at least one authenticated platform:
      </p>
      <p v-else class="text-sm text-gray-600">
        No platforms are authenticated. Please set up authentication first.
      </p>

      <div v-if="hasItems" class="space-y-2">
        <div
          v-for="item in missingAuth"
          :key="item.region"
          class="flex items-center p-2 bg-amber-50 rounded-lg"
        >
          <svg class="w-5 h-5 text-amber-500 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <span class="text-sm text-amber-800">
            {{ item.region === 'local' ? 'Local' : item.region.toUpperCase() }} - no authenticated platforms
          </span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex gap-2">
        <BaseButton variant="secondary" @click="close">
          Dismiss
        </BaseButton>
        <BaseButton variant="primary" @click="goToAuth">
          Set Up Authentication
        </BaseButton>
      </div>
    </template>
  </BaseModal>
</template>
