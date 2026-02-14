<template>
  <div class="forgot-password-container">
    <h1>Forgot Password</h1>
    <p v-if="!submitted">
      Enter your email address and to receive a link to reset your password.
    </p>
    <p v-if="submitted" class="status-message success-message">
      If an account with that email exists, a password reset link has been sent and will arrive within minutes.
      <br />Please check your inbox and also check the spam/junk folder.
    </p>
    <form @submit.prevent="handleSubmit" v-if="!submitted" class="forgot-password-form">
      <div class="form-group">
        <label for="email">Email</label>
        <input type="email" id="email" v-model="email" autocomplete="email" required />
      </div>
      <button type="submit" :disabled="loading">
        {{ loading ? 'Sending...' : 'Send Reset Link' }}
      </button>
      <p v-if="error" class="status-message error-message">{{ error }}</p>
    </form>
    <p class="back-to-login">
    <router-link to="/login">Back to login</router-link>
    </p>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { pluralsync_api, detailed_error_string } from '@/pluralsync_api'

const email = ref('')
const submitted = ref(false)
const loading = ref(false)
const error = ref<string | undefined>(undefined)

const handleSubmit = async () => {
  loading.value = true
  error.value = undefined
  try {
    await pluralsync_api.forgotPassword(email.value)
    submitted.value = true
  } catch (err: any) {
    error.value = detailed_error_string(err)
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.forgot-password-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 80vh;
  padding: 2rem;
}

.forgot-password-form {
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

input[type="email"] {
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
  color: var(--vt-c-white);
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

button:hover:not(:disabled) {
  background-color: var(--color-secondary);
}

.status-message {
  text-align: center;
  margin-top: 1rem;
}

.success-message {
  color: green;
  font-weight: bold;
}

.error-message {
  color: red;
  margin-top: 15px;
}

.back-to-login {
  margin-top: 20px;
  font-size: 14px;
}

.back-to-login a {
  color: var(--color-primary); /* Consistent with primary color */
  text-decoration: none;
}

.back-to-login a:hover {
  text-decoration: underline;
}
</style>
