<script setup lang="ts">
import { computed } from 'vue'
import { useUiStore } from '@/stores/ui'
import BaseModal from '@/components/common/BaseModal.vue'
import BaseButton from '@/components/common/BaseButton.vue'

const uiStore = useUiStore()

const message = computed(() => uiStore.modals.message)

const iconClass = computed(() => {
  switch (message.value.type) {
    case 'error':
      return 'bg-red-100 text-red-600'
    case 'success':
      return 'bg-green-100 text-green-600'
    case 'info':
      return 'bg-blue-100 text-blue-600'
    default:
      return 'bg-amber-100 text-amber-600'
  }
})

function close() {
  uiStore.hideMessage()
}
</script>

<template>
  <BaseModal :title="message.title" @close="close">
    <div class="flex flex-col items-center text-center py-2">
      <!-- Icon -->
      <div class="w-12 h-12 rounded-full flex items-center justify-center mb-4" :class="iconClass">
        <!-- Error icon -->
        <svg v-if="message.type === 'error'" class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="10" stroke-width="2" />
          <line x1="15" y1="9" x2="9" y2="15" stroke-width="2" />
          <line x1="9" y1="9" x2="15" y2="15" stroke-width="2" />
        </svg>

        <!-- Success icon -->
        <svg v-else-if="message.type === 'success'" class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>

        <!-- Info icon -->
        <svg v-else-if="message.type === 'info'" class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="10" stroke-width="2" />
          <line x1="12" y1="16" x2="12" y2="12" stroke-width="2" />
          <line x1="12" y1="8" x2="12.01" y2="8" stroke-width="2" />
        </svg>

        <!-- Warning icon (default) -->
        <svg v-else class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="10" stroke-width="2" />
          <line x1="12" y1="8" x2="12" y2="12" stroke-width="2" />
          <line x1="12" y1="16" x2="12.01" y2="16" stroke-width="2" />
        </svg>
      </div>

      <!-- Message -->
      <p class="text-sm text-gray-600">{{ message.message }}</p>
    </div>

    <template #footer>
      <BaseButton variant="primary" full-width @click="close">
        OK
      </BaseButton>
    </template>
  </BaseModal>
</template>
