<script setup lang="ts">
</script>

<template>
  <div class="fixed inset-0 bg-gray-50 flex flex-col items-center justify-center z-50">
    <div class="flex flex-col items-center gap-4">
      <!-- Logo -->
      <img
        src="@/assets/icon-128.png"
        alt="Columbus"
        class="w-16 h-16 animate-pulse"
      />

      <!-- Loading text -->
      <div class="flex items-center gap-2 text-gray-600">
        <svg
          class="animate-spin w-5 h-5 text-brand"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            class="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            stroke-width="4"
          />
          <path
            class="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
        <span class="text-sm font-medium">Loading...</span>
      </div>
    </div>
  </div>
</template>
