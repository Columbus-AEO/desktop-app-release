<script setup lang="ts">
import { useProductsStore } from '@/stores/products'

const productsStore = useProductsStore()

async function handleChange(event: Event) {
  const productId = (event.target as HTMLSelectElement).value
  if (productId) {
    await productsStore.selectProduct(productId)
  }
}
</script>

<template>
  <div class="bg-white rounded-lg p-4 shadow-sm">
    <label class="block text-sm font-medium text-gray-700 mb-2">
      Product
    </label>
    <select
      :value="productsStore.selectedProductId || ''"
      @change="handleChange"
      class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand focus:border-transparent"
    >
      <option value="">Select a product...</option>

      <!-- Multiple organizations with optgroups -->
      <template v-if="productsStore.hasMultipleOrgs">
        <optgroup
          v-for="(orgData, orgId) in productsStore.productsByOrg"
          :key="orgId"
          :label="orgData.name"
        >
          <option
            v-for="product in orgData.products"
            :key="product.id"
            :value="product.id"
          >
            {{ product.name }}
          </option>
        </optgroup>
      </template>

      <!-- Single organization - flat list -->
      <template v-else>
        <option
          v-for="product in productsStore.products"
          :key="product.id"
          :value="product.id"
        >
          {{ product.name }}
        </option>
      </template>
    </select>
  </div>
</template>
