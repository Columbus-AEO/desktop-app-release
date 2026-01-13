<script setup lang="ts">
import { computed } from 'vue'
import { useProductsStore } from '@/stores/products'
import { useRegionsStore } from '@/stores/regions'
import { useScanStore } from '@/stores/scan'
import BaseButton from '@/components/common/BaseButton.vue'

const productsStore = useProductsStore()
const regionsStore = useRegionsStore()
const scanStore = useScanStore()

const canScan = computed(() => {
  return productsStore.selectedProductId &&
    regionsStore.hasAnyAuth() &&
    productsStore.availableTests > 0 &&
    !scanStore.isScanning
})

async function handleStartScan() {
  await scanStore.startScan()
}
</script>

<template>
  <BaseButton
    variant="primary"
    :disabled="!canScan"
    :loading="scanStore.isScanning"
    full-width
    @click="handleStartScan"
  >
    {{ scanStore.isScanning ? 'Scanning...' : 'Run Scan' }}
  </BaseButton>
</template>
