<script setup lang="ts">
import { useUiStore } from '@/stores/ui'
import BaseModal from '@/components/common/BaseModal.vue'
import BaseButton from '@/components/common/BaseButton.vue'

const uiStore = useUiStore()

function close() {
  uiStore.hideModal('settings')
}

function openRegionAuth() {
  uiStore.hideModal('settings')
  uiStore.showView('region-auth')
}
</script>

<template>
  <BaseModal title="Settings" @close="close">
    <div class="space-y-4">
      <p class="text-sm text-gray-600">
        Manage your platform authentication and region settings.
      </p>

      <BaseButton
        variant="primary"
        full-width
        @click="openRegionAuth"
      >
        Manage Platform Authentication
      </BaseButton>
    </div>
  </BaseModal>
</template>
