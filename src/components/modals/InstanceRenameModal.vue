<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useUiStore } from '@/stores/ui'
import { useInstancesStore } from '@/stores/instances'
import BaseModal from '@/components/common/BaseModal.vue'
import BaseInput from '@/components/common/BaseInput.vue'
import BaseButton from '@/components/common/BaseButton.vue'

const uiStore = useUiStore()
const instancesStore = useInstancesStore()

const newName = ref('')
const isLoading = ref(false)

const instanceId = computed(() => uiStore.modals.instanceRename.instanceId)

// Initialize name when modal opens
watch(instanceId, (id) => {
  if (id) {
    const instance = instancesStore.instances.find(i => i.id === id)
    newName.value = instance?.name || ''
  }
}, { immediate: true })

function close() {
  uiStore.hideInstanceRenameModal()
}

async function save() {
  if (!newName.value.trim() || !instanceId.value) {
    return
  }

  isLoading.value = true
  try {
    await instancesStore.renameInstance(instanceId.value, newName.value.trim())
    close()
  } catch (e) {
    // Error handled in store
  } finally {
    isLoading.value = false
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Enter') {
    save()
  }
}
</script>

<template>
  <BaseModal title="Rename Instance" @close="close">
    <div class="space-y-4">
      <BaseInput
        v-model="newName"
        label="Instance Name"
        placeholder="Enter a name..."
        @keydown="handleKeyDown"
      />
    </div>

    <template #footer>
      <div class="flex gap-2">
        <BaseButton variant="secondary" @click="close">
          Cancel
        </BaseButton>
        <BaseButton
          variant="primary"
          :loading="isLoading"
          :disabled="!newName.trim()"
          @click="save"
        >
          Save
        </BaseButton>
      </div>
    </template>
  </BaseModal>
</template>
