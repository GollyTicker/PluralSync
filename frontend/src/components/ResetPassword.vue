<template>
  <div class="reset-password-container">
    <h1>Reset Password</h1>
    <p v-if="!token">Invalid or missing reset token.</p>
    <form @submit.prevent="handleSubmit" v-if="token && !submitted" class="reset-password-form">
      <div class="form-group">
        <label for="new-password">New Password</label>
        <input type="password" id="new-password" v-model="newPassword" required />
      </div>
      <div class="form-group">
        <label for="confirm-password">Confirm New Password</label>
        <input type="password" id="confirm-password" v-model="confirmPassword" required />
      </div>
      <button type="submit" :disabled="loading">
        {{ loading ? 'Resetting...' : 'Reset Password' }}
      </button>
      <p v-if="error" class="status-message error-message">{{ error }}</p>
    </form>
    <p v-if="submitted" class="status-message success-message">
      Your password has been reset successfully. You can now
      <router-link to="/login">log in</router-link> with your new password.
    </p>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { pluralsync_api, detailed_error_string } from '@/pluralsync_api'

const route = useRoute()
const token = ref<string | undefined>(undefined)
const newPassword = ref('')
const confirmPassword = ref('')
const submitted = ref(false)
const loading = ref(false)
const error = ref<string | undefined>(undefined)

onMounted(() => {
  token.value = route.query.token as string | undefined
  if (!token.value) {
    error.value = 'Invalid or missing reset token.'
  }
})

const handleSubmit = async () => {
  if (!token.value) {
    error.value = 'Reset token is missing.'
    return
  }
  if (newPassword.value !== confirmPassword.value) {
    error.value = 'Passwords do not match.'
    return
  }
  if (!newPassword.value) {
    error.value = 'Password cannot be empty.'
    return
  }

  loading.value = true
  error.value = undefined
  try {
    await pluralsync_api.resetPassword({
      token: { inner: { inner: token.value } },
      new_password: { inner: { inner: newPassword.value } },
    })
    submitted.value = true
  } catch (err: any) {
    error.value = detailed_error_string(err)
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
@import url('../assets/message.css');

.reset-password-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 80vh;
  padding: 2rem;
}

.reset-password-form {
  background: var(--vt-c-white);
  padding: 2rem;
  border-radius: 8px;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
  width: 100%;
  max-width: 400px;
}

h1 {
  text-align: center;
  margin-bottom: 1.5rem;
  color: var(--color-heading);
}

p {
  color: var(--color-text);
  margin-bottom: 15px;
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

input[type='password'] {
  width: 100%;
  padding: 0.8rem;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  box-sizing: border-box;
  background-color: var(--color-background);
  color: var(--color-text);
}

button {
  width: 100%;
  padding: 0.8rem;
  background-color: var(--color-primary);
  color: var(--background-white);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1rem;
  font-weight: bold;
  transition: background-color 0.3s ease;
}

button:disabled {
  background-color: var(--color-border);
  cursor: not-allowed;
}

button:hover {
  background-color: var(--color-secondary);
}
</style>
