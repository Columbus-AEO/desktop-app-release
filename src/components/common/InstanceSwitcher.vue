<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useInstancesStore } from '@/stores/instances'
import { useUiStore } from '@/stores/ui'

const instancesStore = useInstancesStore()
const uiStore = useUiStore()

const isOpen = ref(false)
const dropdownRef = ref<HTMLElement | null>(null)

// Click outside handler
function handleClickOutside(event: MouseEvent) {
  if (dropdownRef.value && !dropdownRef.value.contains(event.target as Node)) {
    isOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})

function toggleDropdown() {
  isOpen.value = !isOpen.value
}

function closeDropdown() {
  isOpen.value = false
}

async function handleSwitch(instanceId: string) {
  if (instanceId !== instancesStore.activeInstanceId) {
    await instancesStore.switchInstance(instanceId)
  }
  closeDropdown()
}

async function handleCreate() {
  await instancesStore.createInstance()
  closeDropdown()
}

async function handleDelete(instanceId: string) {
  const instance = instancesStore.instances.find(i => i.id === instanceId)
  if (!instance) return

  if (confirm(`Delete instance "${instance.name}"?\n\nThis will remove all stored credentials and authentication data for this instance.`)) {
    await instancesStore.deleteInstance(instanceId)
  }
}

function handleRename(instanceId: string) {
  uiStore.showInstanceRenameModal(instanceId)
  closeDropdown()
}
</script>

<template>
  <div ref="dropdownRef" class="relative">
    <!-- Trigger Button -->
    <button
      @click="toggleDropdown"
      class="flex items-center gap-1 px-2 py-1 text-sm text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
    >
      <span class="truncate max-w-[120px]">{{ instancesStore.activeInstanceName }}</span>
      <svg
        class="w-4 h-4 transition-transform"
        :class="{ 'rotate-180': isOpen }"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </button>

    <!-- Dropdown -->
    <div
      v-if="isOpen"
      class="absolute right-0 mt-1 w-56 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-50"
    >
      <!-- Instance List -->
      <div
        v-for="instance in instancesStore.instances"
        :key="instance.id"
        class="flex items-center px-3 py-2 hover:bg-gray-50 cursor-pointer"
        @click="handleSwitch(instance.id)"
      >
        <!-- Check mark -->
        <div class="w-5">
          <svg
            v-if="instance.id === instancesStore.activeInstanceId"
            class="w-4 h-4 text-brand"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        </div>

        <!-- Name -->
        <span class="flex-1 text-sm text-gray-700 truncate">{{ instance.name }}</span>

        <!-- Actions -->
        <div class="flex items-center gap-1">
          <button
            @click.stop="handleRename(instance.id)"
            class="p-1 text-gray-400 hover:text-gray-600 rounded"
            title="Rename"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
          </button>
          <button
            v-if="!instance.is_default"
            @click.stop="handleDelete(instance.id)"
            class="p-1 text-gray-400 hover:text-red-500 rounded"
            title="Delete"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Divider -->
      <div class="border-t border-gray-100 my-1" />

      <!-- Add Instance -->
      <button
        @click="handleCreate"
        class="w-full flex items-center gap-2 px-3 py-2 text-sm text-brand hover:bg-gray-50"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Add Instance
      </button>
    </div>
  </div>
</template>

