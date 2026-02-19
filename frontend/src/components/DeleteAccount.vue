<template>
  <div class="delete-account-container">
    <h1>Delete Account</h1>
    <div class="warning-box">
      <h2 style="margin-top: 0">
        ⚠️ Attention! You are about to delete your account. This action is irreversible ⚠️
      </h2>
      <p>When you delete your account:</p>
      <ul>
        <li>All PluralSync updaters will stop and be permanently removed.</li>
        <li>
          Your fronters symcs managed by PluralSync will NO LONGER work between SimplyPlural,
          PluralKit, Discord, VRChat, or other platforms.
        </li>
        <li>Your website fronting page will stop functioning.</li>
        <li>All saved data will be deleted (authentication tokens, platform credentials, etc.)</li>
        <li>Your account cannot be recovered.</li>
      </ul>
      <p><strong>This is irreversible.</strong></p>
    </div>

    <form @submit.prevent="deleteAccount" v-if="!submitted" class="delete-account-form">
      <div class="form-group">
        <label for="password">Enter your password to confirm:</label>
        <input
          id="password"
          type="password"
          v-model="password"
          placeholder="Your password"
          required
        />
      </div>

      <div class="form-group">
        <label for="confirmation">Type "delete" to confirm account deletion:</label>
        <input
          id="confirmation"
          type="text"
          v-model="confirmation"
          placeholder='Type "delete"'
          required
        />
      </div>

      <p v-if="error" class="status-message error-message">{{ error }}</p>

      <div class="button-group">
        <button type="button" @click="goBack" class="secondary-button">Cancel</button>
        <button type="submit" :disabled="!canSubmit || loading" class="danger-button">
          {{ loading ? 'Deleting...' : 'Delete Account' }}
        </button>
      </div>
    </form>

    <p v-if="submitted" class="status-message success-message">
      Your account has been deleted successfully. Redirecting to home page...
    </p>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import router from '@/router'
import { pluralsync_api, detailed_error_string } from '@/pluralsync_api'

const password = ref('')
const confirmation = ref('')
const error = ref<string | undefined>(undefined)
const loading = ref(false)
const submitted = ref(false)

const canSubmit = () => {
  return password.value.length > 0 && confirmation.value === 'delete'
}

function goBack() {
  router.push('/config')
}

async function deleteAccount() {
  error.value = undefined

  if (confirmation.value !== 'delete') {
    error.value = 'Please type "delete" exactly as shown.'
    return
  }

  if (password.value.length === 0) {
    error.value = 'Please enter your password.'
    return
  }

  loading.value = true

  try {
    await pluralsync_api.delete_user({
      password: { inner: { inner: password.value } },
      confirmation: confirmation.value,
    })

    submitted.value = true

    // Clear authentication and redirect to home
    localStorage.removeItem('jwt')
    setTimeout(() => {
      router.push('/')
    }, 2000)
  } catch (err: any) {
    error.value = detailed_error_string(err)
    loading.value = false
  }
}
</script>

<style scoped>
@import url('../assets/message.css');

.delete-account-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 80vh;
  padding: 2rem;
}

.delete-account-form {
  background: var(--vt-c-white);
  padding: 2rem;
  border-radius: 8px;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
  width: 100%;
  max-width: 500px;
}

h1 {
  text-align: center;
  margin-bottom: 1.5rem;
  color: #d32f2f;
}

p {
  color: var(--color-text);
  margin-bottom: 15px;
}

.warning-box {
  background-color: #ffebee;
  border: 2px solid #d32f2f;
  border-radius: 4px;
  padding: 1.5rem;
  margin-bottom: 2rem;
  width: 100%;
  max-width: 500px;
}

.warning-box h2 {
  color: #d32f2f;
  font-size: 1.1rem;
}

.warning-box ul {
  margin: 1rem 0;
  padding-left: 2rem;
}

.warning-box li {
  margin: 0.5rem 0;
  color: var(--color-text);
}

.form-group {
  margin-bottom: 1.5rem;
  text-align: left;
}

label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 600;
  color: var(--color-text);
}

input[type='password'],
input[type='text'] {
  width: 100%;
  padding: 0.8rem;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  box-sizing: border-box;
  background-color: var(--color-background);
  color: var(--color-text);
}

.button-group {
  display: flex;
  gap: 1rem;
  margin-top: 2rem;
}

button {
  flex: 1;
  padding: 0.8rem;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1rem;
  font-weight: bold;
}

button:disabled {
  background-color: var(--color-border);
  cursor: not-allowed;
}

.secondary-button {
  background-color: var(--color-border);
  color: var(--color-text);
}

.secondary-button:hover:not(:disabled) {
  background-color: #999;
}

.danger-button {
  background-color: #d32f2f;
  color: white;
}

.danger-button:hover:not(:disabled) {
  background-color: #b71c1c;
}
</style>
