import { invoke } from '@tauri-apps/api/core'
import type {
  AuthResult,
  AuthStatus,
  User,
  StatusResult,
  ProductConfig,
  Platform,
  Country,
  AuthInfo,
  DailyUsage,
  PromptData,
  Instance,
  ScanProgress,
  ScheduleInfo,
  PaaResult,
  StartScanParams
} from '@/types'

export function useTauriCommands() {
  // ==================== Authentication ====================
  const login = (email: string, password: string) =>
    invoke<AuthResult>('login', { email, password })

  const loginWithGoogle = () =>
    invoke<User>('login_with_google')

  const logout = () =>
    invoke('logout')

  const getAuthStatus = () =>
    invoke<AuthStatus>('get_auth_status')

  // ==================== Products ====================
  const getStatus = () =>
    invoke<StatusResult>('get_status')

  const getProductConfig = (productId: string) =>
    invoke<ProductConfig>('get_product_config', { productId })

  const setProductConfig = (
    productId: string,
    readyPlatforms: string[],
    samplesPerPrompt: number,
    autoRunEnabled: boolean,
    scansPerDay: number,
    timeWindowStart: number,
    timeWindowEnd: number
  ) =>
    invoke('set_product_config', {
      productId,
      readyPlatforms,
      samplesPerPrompt,
      autoRunEnabled,
      scansPerDay,
      timeWindowStart,
      timeWindowEnd
    })

  const getLastProductId = () =>
    invoke<string | null>('get_last_product_id')

  const setLastProductId = (productId: string) =>
    invoke('set_last_product_id', { productId })

  // ==================== Scanning ====================
  const startScan = (params: StartScanParams) =>
    invoke('start_scan', params)

  const cancelScan = () =>
    invoke('cancel_scan')

  const getScanProgress = () =>
    invoke<ScanProgress>('get_scan_progress')

  const isScanRunning = () =>
    invoke<boolean>('is_scan_running')

  // ==================== Regions & Proxy ====================
  const fetchProxyConfig = () =>
    invoke<Country[]>('fetch_proxy_config')

  const getConfiguredProxyCountries = () =>
    invoke<string[]>('get_configured_proxy_countries')

  const getCountryPlatformAuth = (countryCode: string, platform: string) =>
    invoke<AuthInfo | null>('get_country_platform_auth', { countryCode, platform })

  const setCountryPlatformAuth = (countryCode: string, platform: string, authenticated: boolean) =>
    invoke('set_country_platform_auth', { countryCode, platform, authenticated })

  const setPlatformAuthStatus = (countryCode: string, platform: string, authenticated: boolean) =>
    invoke('set_platform_auth_status', { countryCode, platform, authenticated })

  const openCountryLogin = (countryCode: string, platform: string, visible: boolean) =>
    invoke('open_country_login', { countryCode, platform, visible })

  const openMagicLink = (countryCode: string, url: string) =>
    invoke('open_magic_link', { countryCode, url })

  const getPromptRegions = (productId: string) =>
    invoke<string[]>('get_prompt_regions', { productId })

  const getPromptTargetRegions = (productId: string) =>
    invoke<Record<string, string[]>>('get_prompt_target_regions', { productId })

  // ==================== Platforms ====================
  const getAiPlatforms = (forceRefresh = false) =>
    invoke<Platform[]>('get_ai_platforms', { forceRefresh })

  const openUrlInBrowser = (url: string) =>
    invoke('open_url_in_browser', { url })

  // ==================== Instances ====================
  const listInstances = () =>
    invoke<Instance[]>('list_instances')

  const getActiveInstance = () =>
    invoke<Instance | null>('get_active_instance')

  const createInstance = (name?: string | null) =>
    invoke<Instance>('create_instance', { name })

  const deleteInstance = (instanceId: string) =>
    invoke('delete_instance', { instanceId })

  const renameInstance = (instanceId: string, newName: string) =>
    invoke('rename_instance', { instanceId, newName })

  const switchInstance = (instanceId: string) =>
    invoke('switch_instance', { instanceId })

  // ==================== Settings ====================
  const getAutostartEnabled = () =>
    invoke<boolean>('get_autostart_enabled')

  const setAutostartEnabled = (enabled: boolean) =>
    invoke('set_autostart_enabled', { enabled })

  const getScheduleInfo = (productId: string) =>
    invoke<ScheduleInfo>('get_schedule_info', { productId })

  // ==================== API & Usage ====================
  const checkDailyUsage = () =>
    invoke<DailyUsage>('check_daily_usage')

  const fetchExtensionPrompts = (productId: string) =>
    invoke<PromptData>('fetch_extension_prompts', { productId })

  // ==================== Onboarding ====================
  const isOnboardingCompleted = () =>
    invoke<boolean>('is_onboarding_completed')

  const setOnboardingCompleted = (completed: boolean) =>
    invoke('set_onboarding_completed', { completed })

  // ==================== PAA/Keyword Discovery ====================
  const startPaaDiscovery = (productId: string, seedKeyword: string) =>
    invoke<PaaResult>('start_paa_discovery', { productId, seedKeyword })

  return {
    // Auth
    login,
    loginWithGoogle,
    logout,
    getAuthStatus,
    // Products
    getStatus,
    getProductConfig,
    setProductConfig,
    getLastProductId,
    setLastProductId,
    // Scanning
    startScan,
    cancelScan,
    getScanProgress,
    isScanRunning,
    // Regions & Proxy
    fetchProxyConfig,
    getConfiguredProxyCountries,
    getCountryPlatformAuth,
    setCountryPlatformAuth,
    setPlatformAuthStatus,
    openCountryLogin,
    openMagicLink,
    getPromptRegions,
    getPromptTargetRegions,
    // Platforms
    getAiPlatforms,
    openUrlInBrowser,
    // Instances
    listInstances,
    getActiveInstance,
    createInstance,
    deleteInstance,
    renameInstance,
    switchInstance,
    // Settings
    getAutostartEnabled,
    setAutostartEnabled,
    getScheduleInfo,
    // API & Usage
    checkDailyUsage,
    fetchExtensionPrompts,
    // Onboarding
    isOnboardingCompleted,
    setOnboardingCompleted,
    // PAA
    startPaaDiscovery
  }
}
