// User and Authentication types
export interface User {
  id: string
  email: string
}

export interface AuthStatus {
  authenticated: boolean
  user: User | null
}

export interface AuthResult {
  user: User
  token?: string
}

// Organization and Product types
export interface Organization {
  id: string
  name: string
}

export interface Product {
  id: string
  name: string
  organizationId: string
  organizationName: string
}

export interface ProductConfig {
  samples_per_prompt: number
  auto_run_enabled: boolean
  scans_per_day: number
  time_window_start: number
  time_window_end: number
  ready_platforms?: string[]
}

export interface StatusResult {
  products: Product[]
  organizations: Organization[]
}

// Platform types
export interface Platform {
  id: string
  name: string
  logo_url: string | null
  website_url: string
}

// Region and Country types
export interface Country {
  code: string
  name: string
  flag_emoji: string
}

export const LOCAL_REGION: Country = {
  code: 'local',
  name: 'Local (Your Location)',
  flag_emoji: '🏠'
}

export interface AuthInfo {
  is_authenticated?: boolean
  isAuthenticated?: boolean
}

// Daily Usage types
export interface DailyUsage {
  current: number
  limit: number
  remaining: number
  effectiveRemaining: number
  pendingEvaluations: number
  isUnlimited: boolean
  plan: string
}

export interface PromptData {
  prompts: unknown[]
  totalPrompts: number
  quota?: {
    promptsUsedToday: number
    promptsPerDay: number
    promptsRemaining?: number
    effectiveRemaining?: number
    pendingEvaluations?: number
    isUnlimited?: boolean
    plan?: string
  }
}

// Instance types
export interface Instance {
  id: string
  name: string
  is_default: boolean
}

// Scan types
export type ScanPhase = 'initializing' | 'submitting' | 'waiting' | 'collecting' | 'complete' | 'cancelled'

export interface PlatformState {
  status: string
  total: number
  submitted: number
  collected: number
}

export interface ScanProgress {
  phase: ScanPhase
  platforms: Record<string, PlatformState>
  countdownSeconds?: number | null
}

export interface ScanComplete {
  total_prompts: number
  successful_prompts: number
  mention_rate: number
  citation_rate: number
}

export interface StartScanParams {
  productId: string
  samplesPerPrompt: number
  platforms: string[]
  maxTests: number | null
  [key: string]: unknown
}

// Schedule types
export interface ScheduleInfo {
  next_scan_hour: number | null
  scans_completed_today: number
  scans_total_today: number
}

// Update types
export interface UpdateInfo {
  version: string
  downloadAndInstall: (progressCallback: (progress: UpdateProgress) => void) => Promise<void>
}

export interface UpdateProgress {
  event: 'Started' | 'Progress' | 'Finished'
  data: {
    contentLength?: number
    chunkLength?: number
  }
}

// PAA/Keyword Discovery types
export interface PaaResult {
  success: boolean
  code?: 'RATE_LIMIT_EXCEEDED' | 'GOOGLE_AUTH_REQUIRED' | 'NO_PAA_FOUND'
  message?: string
  error?: string
}

export interface PaaProgress {
  current: number
  message?: string
}

// Modal types
export type MessageType = 'info' | 'warning' | 'error' | 'success'

export interface MessageModalState {
  visible: boolean
  title: string
  message: string
  type: MessageType
}

export interface InstanceRenameModalState {
  visible: boolean
  instanceId: string | null
}

// View types
export type ViewName = 'login' | 'onboarding' | 'region-auth' | 'main' | 'scanning' | 'complete'
