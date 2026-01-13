<script setup lang="ts">
import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useTauriCommands } from '@/composables/useTauriCommands'
import BaseButton from '@/components/common/BaseButton.vue'
import BaseInput from '@/components/common/BaseInput.vue'
import { SIGNUP_URL } from '@/utils/constants'

const authStore = useAuthStore()
const commands = useTauriCommands()

const email = ref('')
const password = ref('')
const error = ref('')
const isLoading = ref(false)
const isGoogleLoading = ref(false)

async function handleLogin() {
  if (!email.value || !password.value) {
    error.value = 'Please enter email and password'
    return
  }

  isLoading.value = true
  error.value = ''

  try {
    await authStore.login(email.value, password.value)
  } catch (e) {
    error.value = String(e)
  } finally {
    isLoading.value = false
  }
}

async function handleGoogleLogin() {
  isGoogleLoading.value = true
  error.value = ''

  try {
    await authStore.loginWithGoogle()
  } catch (e) {
    error.value = 'Failed to start Google login: ' + e
  } finally {
    isGoogleLoading.value = false
  }
}

function openSignup() {
  commands.openUrlInBrowser(SIGNUP_URL)
}
</script>

<template>
  <div class="h-full flex flex-col items-center justify-center p-6 bg-gradient-to-b from-gray-50 to-gray-100">
    <div class="w-full max-w-sm">
      <!-- Logo -->
      <div class="flex flex-col items-center mb-8">
        <img
          src="@/assets/icon-128.png"
          alt="Columbus"
          class="w-16 h-16 mb-4"
        />
        <h1 class="text-2xl font-bold text-gray-900">Columbus</h1>
        <p class="text-sm text-gray-500">AI Brand Monitor</p>
      </div>

      <!-- Login Form -->
      <form @submit.prevent="handleLogin" class="space-y-4">
        <BaseInput
          v-model="email"
          type="email"
          label="Email"
          placeholder="Enter your email"
          required
        />

        <BaseInput
          v-model="password"
          type="password"
          label="Password"
          placeholder="Enter your password"
          required
        />

        <!-- Error message -->
        <div
          v-if="error"
          class="p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600"
        >
          {{ error }}
        </div>

        <BaseButton
          type="submit"
          variant="primary"
          :loading="isLoading"
          :disabled="isLoading || isGoogleLoading"
          full-width
        >
          Sign In
        </BaseButton>
      </form>

      <!-- Divider -->
      <div class="relative my-6">
        <div class="absolute inset-0 flex items-center">
          <div class="w-full border-t border-gray-200" />
        </div>
        <div class="relative flex justify-center text-sm">
          <span class="px-2 bg-gradient-to-b from-gray-50 to-gray-100 text-gray-500">or</span>
        </div>
      </div>

      <!-- Google Login -->
      <BaseButton
        type="button"
        variant="secondary"
        :loading="isGoogleLoading"
        :disabled="isLoading || isGoogleLoading"
        full-width
        @click="handleGoogleLogin"
      >
        <svg class="w-5 h-5 mr-2" viewBox="0 0 24 24">
          <path
            fill="currentColor"
            d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
          />
          <path
            fill="currentColor"
            d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
          />
          <path
            fill="currentColor"
            d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
          />
          <path
            fill="currentColor"
            d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
          />
        </svg>
        Continue with Google
      </BaseButton>

      <!-- Sign up link -->
      <p class="mt-6 text-center text-sm text-gray-500">
        Don't have an account?
        <button
          type="button"
          @click="openSignup"
          class="text-brand hover:text-brand-dark font-medium"
        >
          Sign up
        </button>
      </p>
    </div>
  </div>
</template>
