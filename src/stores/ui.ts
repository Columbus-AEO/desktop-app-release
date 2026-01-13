import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import type { ViewName, MessageModalState, InstanceRenameModalState, MessageType, UpdateInfo } from '@/types'

export const useUiStore = defineStore('ui', () => {
  // View state
  const currentView = ref<ViewName>('login')
  const isInitializing = ref(true)
  const appVersion = ref<string | null>(null)

  // Update state
  const updateAvailable = ref<UpdateInfo | null>(null)
  const isUpdating = ref(false)
  const updateProgress = ref(0)
  const updateButtonText = computed(() => {
    if (isUpdating.value) {
      return updateProgress.value > 0
        ? `Downloading... ${updateProgress.value}%`
        : 'Downloading...'
    }
    return 'Install & Restart'
  })

  // Modal states
  const modals = ref({
    settings: false,
    addRegion: false,
    authRequired: false,
    instanceRename: { visible: false, instanceId: null } as InstanceRenameModalState,
    message: { visible: false, title: '', message: '', type: 'info' as MessageType } as MessageModalState,
    keywordDiscovery: false
  })

  // Auth required modal data
  const authRequiredData = ref<{ region: string; platforms: string[] }[]>([])

  // Actions
  function showView(view: ViewName) {
    currentView.value = view
  }

  function setInitializing(value: boolean) {
    isInitializing.value = value
  }

  async function loadAppVersion() {
    try {
      appVersion.value = await getVersion()
    } catch (error) {
      console.error('Failed to get app version:', error)
    }
  }

  async function checkForUpdates() {
    try {
      console.log('[Updater] Checking for updates...')
      const update = await check()
      if (update) {
        console.log(`[Updater] Update available: ${update.version}`)
        updateAvailable.value = update as UpdateInfo
      } else {
        console.log('[Updater] No updates available')
      }
    } catch (error) {
      console.log('[Updater] Update check failed:', error)
    }
  }

  async function installUpdate() {
    if (!updateAvailable.value || isUpdating.value) return

    isUpdating.value = true
    updateProgress.value = 0

    try {
      let downloaded = 0
      let contentLength = 0

      await updateAvailable.value.downloadAndInstall((progress) => {
        if (progress.event === 'Started') {
          contentLength = progress.data.contentLength || 0
          updateProgress.value = 0
        } else if (progress.event === 'Progress') {
          downloaded += progress.data.chunkLength || 0
          if (contentLength > 0) {
            updateProgress.value = Math.round((downloaded / contentLength) * 100)
          }
        } else if (progress.event === 'Finished') {
          updateProgress.value = 100
        }
      })

      await relaunch()
    } catch (error) {
      console.error('Update failed:', error)
      isUpdating.value = false
      showMessage('Update failed: ' + error, 'Update Error', 'error')
    }
  }

  function dismissUpdate() {
    updateAvailable.value = null
  }

  // Modal actions
  function showModal(modal: keyof typeof modals.value) {
    if (modal === 'instanceRename' || modal === 'message') return // Use specific functions
    modals.value[modal] = true
  }

  function hideModal(modal: keyof typeof modals.value) {
    if (modal === 'instanceRename') {
      modals.value.instanceRename = { visible: false, instanceId: null }
    } else if (modal === 'message') {
      modals.value.message = { visible: false, title: '', message: '', type: 'info' }
    } else {
      modals.value[modal] = false
    }
  }

  function showMessage(message: string, title = 'Notice', type: MessageType = 'warning') {
    modals.value.message = { visible: true, title, message, type }
  }

  function hideMessage() {
    modals.value.message = { visible: false, title: '', message: '', type: 'info' }
  }

  function showInstanceRenameModal(instanceId: string) {
    modals.value.instanceRename = { visible: true, instanceId }
  }

  function hideInstanceRenameModal() {
    modals.value.instanceRename = { visible: false, instanceId: null }
  }

  function showAuthRequiredModal(missingAuth: { region: string; platforms: string[] }[]) {
    authRequiredData.value = missingAuth
    modals.value.authRequired = true
  }

  return {
    // State
    currentView,
    isInitializing,
    appVersion,
    updateAvailable,
    isUpdating,
    updateProgress,
    updateButtonText,
    modals,
    authRequiredData,

    // Actions
    showView,
    setInitializing,
    loadAppVersion,
    checkForUpdates,
    installUpdate,
    dismissUpdate,
    showModal,
    hideModal,
    showMessage,
    hideMessage,
    showInstanceRenameModal,
    hideInstanceRenameModal,
    showAuthRequiredModal
  }
})
