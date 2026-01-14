export const DASHBOARD_URL = 'https://columbus-aeo.com/dashboard'
export const SIGNUP_URL = 'https://columbus-aeo.com/signup'
export const TUTORIAL_URL = 'https://youtu.be/I8Fo_jJYZyI'

// Supabase configuration
export const SUPABASE_URL = 'https://yvhzxuoqodutmllfhcsa.supabase.co'
export const SUPABASE_ANON_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Inl2aHp4dW9xb2R1dG1sbGZoY3NhIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NjM5MDMwOTIsImV4cCI6MjA3OTQ3OTA5Mn0.UxDcyOAGSGKBW26ElQXAyiVC6GRicphIVcrMs8tdkRI'

// Phase display text mapping
export const PHASE_DISPLAY_TEXT: Record<string, string> = {
  initializing: 'Initializing...',
  submitting: 'Submitting prompts...',
  waiting: 'Waiting for responses...',
  collecting: 'Collecting responses...',
  complete: 'Complete',
  cancelled: 'Cancelled'
}

// Default fallback platforms
export const DEFAULT_PLATFORMS = ['chatgpt', 'claude', 'gemini', 'perplexity']

export const DEFAULT_PLATFORM_URLS: Record<string, string> = {
  chatgpt: 'https://chat.openai.com',
  claude: 'https://claude.ai',
  gemini: 'https://gemini.google.com',
  perplexity: 'https://perplexity.ai'
}

export const DEFAULT_PLATFORM_NAMES: Record<string, string> = {
  chatgpt: 'ChatGPT',
  claude: 'Claude',
  gemini: 'Gemini',
  perplexity: 'Perplexity'
}

// Utility function
export function capitalizeFirst(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1)
}
