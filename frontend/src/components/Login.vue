<template>
  <div class="login-container">
    <h1>Login</h1>
    <form @submit.prevent="login" class="login-form">
      <div class="form-group">
        <label for="email">Email</label>
        <input type="email" id="email" v-model="email" autocomplete="email" />
      </div>
      <div class="form-group">
        <label for="password">Password</label>
        <input type="password" id="password" v-model="password" autocomplete="password" />
      </div>
      <button type="submit">Login</button>
      <button @click="register" type="button" class="register-button">Register</button>
      <p class="forgot-password-link-container">
        <router-link to="/forgot-password" class="forgot-password-link">
          Forgot Password?
        </router-link>
      </p>
    </form>
    <p v-if="status" class="status-message">{{ status }}</p>
    <p style="margin: 1em">ℹ️ PluralSync was formerly named SP2Any. ℹ️</p>
  </div>
</template>

<script setup lang="ts">
defineProps({})

import { ref, type Ref } from 'vue'
import router from '@/router'
import type { UserLoginCredentials } from '@/pluralsync.bindings'
import { detailed_error_string, pluralsync_api } from '@/pluralsync_api'

const email: Ref<string> = ref('')
const password: Ref<string> = ref('')
const status: Ref<string> = ref('')

const login = async () => {
  if (!email.value || !password.value) {
    status.value = 'Email/Password cannot be empty.'
    return
  }
  const creds = {
    email: { inner: email.value },
    password: { inner: { inner: password.value } },
  } as UserLoginCredentials

  try {
    await pluralsync_api.login(creds)
    console.log('Login successful!')
    status.value = ''
    router.push('/status')
  } catch (err: any) {
    status.value = 'Login failed:' + detailed_error_string(err)
    console.error('Login failed:', err)
  }
}

const register = async () => {
  if (!email.value || !password.value) {
    status.value = 'Email/Password cannot be empty.'
    return
  }
  const creds = {
    email: { inner: email.value },
    password: { inner: { inner: password.value } },
  } as UserLoginCredentials

  try {
    status.value = 'Sending registration request...'
    await pluralsync_api.register(creds)
    status.value =
      'Registering your account... A verification link has been sent to your email. Click on it to activate your account!'
  } catch (err: any) {
    status.value = 'Registration failed: ' + detailed_error_string(err)
    console.error('Registration failed:', err)
  }
}
</script>

<style scoped>
@import url('../assets/message.css');

.login-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 80vh;
  padding: 2rem;
}

.login-form {
  background: #fff;
  padding: 2rem;
  border-radius: 8px;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
  width: 100%;
  max-width: 400px;
}

h1 {
  text-align: center;
  margin-bottom: 1.5rem;
}

.form-group {
  margin-bottom: 1.5rem;
}

label {
  display: block;
  margin-bottom: 0.5rem;
  font-weight: 600;
}

input {
  width: 100%;
  padding: 0.8rem;
  border: 1px solid #ccc;
  border-radius: 4px;
  box-sizing: border-box;
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
}

button:hover {
  background-color: var(--color-secondary);
}

.register-button {
  margin-top: 0.5rem;
  background-color: var(--color-background-soft);
}

.register-button:hover {
  background-color: var(--color-background-mute);
}

.forgot-password-link-container {
  text-align: center;
  margin-top: 1rem;
}

.forgot-password-link {
  color: var(--color-primary);
  text-decoration: none;
  font-size: 0.9rem;
}

.forgot-password-link:hover {
  text-decoration: underline;
}
</style>
