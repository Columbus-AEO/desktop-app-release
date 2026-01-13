<script setup lang="ts">
import { computed } from 'vue'
import { useRegionsStore } from '@/stores/regions'
import { usePlatformsStore } from '@/stores/platforms'
import BaseButton from '@/components/common/BaseButton.vue'

const props = defineProps<{
  platformId: string
  region: string
}>()

const emit = defineEmits<{
  login: [platform: string]
  toggle: [platform: string]
}>()

const regionsStore = useRegionsStore()
const platformsStore = usePlatformsStore()

const isAuthenticated = computed(() =>
  regionsStore.isPlatformAuthForRegion(props.region, props.platformId)
)

const platformName = computed(() =>
  platformsStore.getPlatformName(props.platformId)
)

const platformLogo = computed(() =>
  platformsStore.getPlatformLogo(props.platformId)
)

function handleLogin() {
  emit('login', props.platformId)
}

function handleToggle() {
  emit('toggle', props.platformId)
}
</script>

<template>
  <div
    class="flex items-center justify-between p-3 bg-white rounded-lg shadow-sm border"
    :class="[isAuthenticated ? 'border-green-200' : 'border-gray-200']"
  >
    <div class="flex items-center gap-3">
      <!-- Platform Icon -->
      <div
        v-if="platformLogo"
        class="w-8 h-8 rounded-lg overflow-hidden bg-gray-100"
      >
        <img :src="platformLogo" :alt="platformName" class="w-full h-full object-cover" />
      </div>
      <div
        v-else
        class="w-8 h-8 rounded-lg"
        :class="`platform-${platformId}`"
      />

      <!-- Info -->
      <div>
        <p class="text-sm font-medium text-gray-900">{{ platformName }}</p>
        <p
          class="text-xs"
          :class="[isAuthenticated ? 'text-green-600' : 'text-gray-400']"
        >
          {{ isAuthenticated ? 'Authenticated' : 'Not authenticated' }}
        </p>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <BaseButton
        :variant="isAuthenticated ? 'secondary' : 'primary'"
        size="sm"
        @click="handleLogin"
      >
        {{ isAuthenticated ? 'Re-auth' : 'Login' }}
      </BaseButton>

      <button
        @click="handleToggle"
        class="w-8 h-8 rounded-lg flex items-center justify-center transition-colors"
        :class="[
          isAuthenticated
            ? 'bg-green-100 text-green-600 hover:bg-green-200'
            : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
        ]"
        :title="isAuthenticated ? 'Mark as not logged in' : 'Mark as logged in'"
      >
        {{ isAuthenticated ? '✓' : '○' }}
      </button>
    </div>
  </div>
</template>
