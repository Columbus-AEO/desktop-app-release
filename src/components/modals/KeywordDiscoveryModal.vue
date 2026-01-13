<script setup lang="ts">
import { ref } from 'vue'
import { useUiStore } from '@/stores/ui'
import { useProductsStore } from '@/stores/products'
import { useTauriCommands } from '@/composables/useTauriCommands'
import BaseModal from '@/components/common/BaseModal.vue'
import BaseInput from '@/components/common/BaseInput.vue'
import BaseButton from '@/components/common/BaseButton.vue'
import ProgressBar from '@/components/common/ProgressBar.vue'

const uiStore = useUiStore()
const productsStore = useProductsStore()
const commands = useTauriCommands()

const seedKeyword = ref('')
const isDiscovering = ref(false)
const progress = ref(0)

function close(force = false) {
  if (isDiscovering.value && !force) return
  uiStore.hideModal('keywordDiscovery')
  seedKeyword.value = ''
  progress.value = 0
}

async function startDiscovery() {
  if (!seedKeyword.value.trim()) {
    close(true)
    uiStore.showMessage('Please enter a seed keyword.', 'Keyword Required', 'warning')
    return
  }

  if (!productsStore.selectedProductId) {
    close(true)
    uiStore.showMessage('Please select a product first.', 'No Product Selected', 'warning')
    return
  }

  isDiscovering.value = true
  progress.value = 10

  try {
    const result = await commands.startPaaDiscovery(
      productsStore.selectedProductId,
      seedKeyword.value.trim()
    )

    close(true)

    if (result.code === 'RATE_LIMIT_EXCEEDED') {
      uiStore.showMessage(
        result.message || 'Rate limit exceeded. Please try again later.',
        'Rate Limited',
        'warning'
      )
    } else if (result.code === 'GOOGLE_AUTH_REQUIRED') {
      uiStore.showMessage(
        result.message || 'Please authenticate Google AI Overview first.',
        'Authentication Required',
        'warning'
      )
    } else if (result.code === 'NO_PAA_FOUND') {
      uiStore.showMessage(
        result.message || 'No "People Also Ask" section found. Try a different keyword.',
        'No Results',
        'info'
      )
    } else if (!result.success && result.error) {
      uiStore.showMessage(
        result.message || result.error || 'Failed to discover keywords.',
        'Error',
        'error'
      )
    } else if (result.success) {
      uiStore.showMessage(
        'The discovered questions will be analyzed and should be available in your Columbus Dashboard shortly.',
        'Discovery Complete',
        'success'
      )
    }
  } catch (e) {
    close(true)
    uiStore.showMessage(
      String(e) || 'Failed to discover keywords',
      'Error',
      'error'
    )
  } finally {
    isDiscovering.value = false
    progress.value = 0
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !isDiscovering.value) {
    startDiscovery()
  }
}
</script>

<template>
  <BaseModal title="Keyword Discovery" :show-close="!isDiscovering" @close="close()">
    <div class="space-y-4">
      <p class="text-sm text-gray-600">
        Enter a seed keyword to discover related questions from Google's "People Also Ask" section.
      </p>

      <BaseInput
        v-model="seedKeyword"
        label="Seed Keyword"
        placeholder="e.g., best CRM software"
        :disabled="isDiscovering"
        @keydown="handleKeyDown"
      />

      <!-- Progress -->
      <div v-if="isDiscovering" class="space-y-2">
        <ProgressBar :value="progress" show-label />
        <p class="text-xs text-center text-gray-500">Discovering keywords...</p>
      </div>
    </div>

    <template #footer>
      <div class="flex gap-2">
        <BaseButton
          variant="secondary"
          :disabled="isDiscovering"
          @click="close()"
        >
          Cancel
        </BaseButton>
        <BaseButton
          variant="primary"
          :loading="isDiscovering"
          :disabled="!seedKeyword.trim() || isDiscovering"
          @click="startDiscovery"
        >
          <span v-if="!isDiscovering">Start Discovery</span>
          <span v-else>Discovering...</span>
        </BaseButton>
      </div>
    </template>
  </BaseModal>
</template>
