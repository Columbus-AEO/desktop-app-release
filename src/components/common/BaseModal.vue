<script setup lang="ts">
interface Props {
  title?: string
  showClose?: boolean
}

withDefaults(defineProps<Props>(), {
  title: '',
  showClose: true
})

const emit = defineEmits<{
  close: []
}>()
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <!-- Overlay -->
    <div
      class="absolute inset-0 bg-black/50 backdrop-blur-sm"
      @click="emit('close')"
    />

    <!-- Modal -->
    <div class="relative bg-white rounded-xl shadow-xl max-w-md w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div v-if="title || showClose" class="flex items-center justify-between p-4 border-b border-gray-100">
        <h3 v-if="title" class="text-lg font-semibold text-gray-900">
          {{ title }}
        </h3>
        <div v-else />
        <button
          v-if="showClose"
          @click="emit('close')"
          class="text-gray-400 hover:text-gray-600 transition-colors"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="p-4 overflow-y-auto flex-1">
        <slot />
      </div>

      <!-- Footer -->
      <div v-if="$slots.footer" class="p-4 border-t border-gray-100 bg-gray-50">
        <slot name="footer" />
      </div>
    </div>
  </div>
</template>
