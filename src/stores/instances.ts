import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useRegionsStore } from './regions'
import { useUiStore } from './ui'
import type { Instance } from '@/types'

export const useInstancesStore = defineStore('instances', () => {
  const commands = useTauriCommands()

  // State
  const instances = ref<Instance[]>([])
  const activeInstanceId = ref<string | null>(null)
  const isLoading = ref(false)

  // Getters
  const activeInstance = computed(() =>
    instances.value.find(i => i.id === activeInstanceId.value) || null
  )

  const activeInstanceName = computed(() =>
    activeInstance.value?.name || 'Default'
  )

  // Actions
  async function loadInstances() {
    try {
      instances.value = await commands.listInstances()
      const active = await commands.getActiveInstance()
      activeInstanceId.value = active?.id || null
      console.log(`Loaded ${instances.value.length} instances, active: ${activeInstanceId.value}`)
    } catch (error) {
      console.error('Failed to load instances:', error)
      instances.value = []
      activeInstanceId.value = null
    }
  }

  async function switchInstance(instanceId: string) {
    const regionsStore = useRegionsStore()

    try {
      await commands.switchInstance(instanceId)
      activeInstanceId.value = instanceId
      console.log(`Switched to instance: ${instanceId}`)

      // Reload instance-scoped data
      await regionsStore.loadPlatformAuthStatus()
    } catch (error) {
      console.error('Failed to switch instance:', error)
      const uiStore = useUiStore()
      uiStore.showMessage('Failed to switch instance: ' + error, 'Error', 'error')
    }
  }

  async function createInstance(name?: string) {
    try {
      const newInstance = await commands.createInstance(name || null)
      instances.value.push(newInstance)
      console.log(`Created instance: ${newInstance.name} (${newInstance.id})`)

      // Switch to the new instance
      await switchInstance(newInstance.id)
      return newInstance
    } catch (error) {
      console.error('Failed to create instance:', error)
      const uiStore = useUiStore()
      uiStore.showMessage('Failed to create instance: ' + error, 'Error', 'error')
      throw error
    }
  }

  async function deleteInstance(instanceId: string) {
    const instance = instances.value.find(i => i.id === instanceId)
    if (!instance) return

    try {
      await commands.deleteInstance(instanceId)
      instances.value = instances.value.filter(i => i.id !== instanceId)
      console.log(`Deleted instance: ${instanceId}`)

      // If deleted the active instance, switch to default
      if (instanceId === activeInstanceId.value) {
        const defaultInstance = instances.value.find(i => i.is_default)
        if (defaultInstance) {
          await switchInstance(defaultInstance.id)
        }
      }
    } catch (error) {
      console.error('Failed to delete instance:', error)
      const uiStore = useUiStore()
      uiStore.showMessage('Failed to delete instance: ' + error, 'Error', 'error')
    }
  }

  async function renameInstance(instanceId: string, newName: string) {
    try {
      await commands.renameInstance(instanceId, newName)

      // Update local state
      const instance = instances.value.find(i => i.id === instanceId)
      if (instance) {
        instance.name = newName
      }

      console.log(`Renamed instance ${instanceId} to: ${newName}`)
    } catch (error) {
      console.error('Failed to rename instance:', error)
      const uiStore = useUiStore()
      uiStore.showMessage('Failed to rename instance: ' + error, 'Error', 'error')
      throw error
    }
  }

  return {
    // State
    instances,
    activeInstanceId,
    isLoading,

    // Getters
    activeInstance,
    activeInstanceName,

    // Actions
    loadInstances,
    switchInstance,
    createInstance,
    deleteInstance,
    renameInstance
  }
})
