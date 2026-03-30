<template>
  <div class="login-container">
    <h1>PluralSync</h1>
    <p class="link-container">
      <router-link to="/about" class="link"> What is this? </router-link>
    </p>
    <form @submit.prevent="login" class="login-form">
      <h2>Login</h2>
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
      <p class="link-container forgot-password">
        <router-link to="/forgot-password" class="link"> Forgot Password? </router-link>
      </p>
    </form>
    <p v-if="status" class="status-message">{{ status }}</p>
    <p style="margin: 1em">✨ We've added support for syncing from PluralKit ✨</p>
    <p style="margin: 1em">
      ❗ Please read this post by the local system manager (Ampersand) developer:
      <a href="https://ampersand.moe/blog/rumors.html">Clearing up rumors (no, I won't be nice)</a>
    </p>
    <p style="margin: 1em">
      Announcement:
      <a href="/announcements#2026-03-simply_plural_discontinuation">
        Regarding the Discontinuation of SimplyPlural
      </a>
    </p>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, type Ref } from 'vue'
import router from '@/router'
import type { AxiosError } from 'axios'
import type { UserLoginCredentials } from '@/pluralsync.bindings'
import { detailed_error_string, pluralsync_api } from '@/pluralsync_api'
import { loggedIn } from '@/jwt'

onMounted(() => {
  if (loggedIn.value) {
    router.push('/status')
  }
})

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
    password: { inner: { inner: password.value } }
  } as UserLoginCredentials

  try {
    await pluralsync_api.login(creds)
    console.log('Login successful!')
    status.value = ''
    router.push('/status')
  } catch (err: unknown) {
    status.value = 'Login failed:' + detailed_error_string(err as AxiosError | Error)
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
    password: { inner: { inner: password.value } }
  } as UserLoginCredentials

  try {
    status.value = 'Sending registration request...'
    await pluralsync_api.register(creds)
    status.value =
      'Registering your account... A verification link has been sent to your email. Click on it to activate your account!'
  } catch (err: unknown) {
    status.value = 'Registration failed: ' + detailed_error_string(err as AxiosError | Error)
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
  margin-bottom: 0.25rem;
}

h2 {
  text-align: center;
  margin-bottom: 1.5rem;
  font-size: 1.5rem;
}

.link-container {
  text-align: center;
  margin-bottom: 2rem;
}

/* Mobile optimization: reduce vertical spacing on smaller screens */
@media (max-width: 768px) {
  .login-container {
    min-height: auto;
    padding: 1rem;
    justify-content: flex-start;
  }

  .login-form {
    padding: 1.5rem;
  }

  h1 {
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    font-size: 1.75rem;
  }

  h2 {
    margin-bottom: 1rem;
    font-size: 1.25rem;
  }

  .link-container {
    margin-bottom: 1rem;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  label,
  input {
    font-size: 1rem; /* Prevents zoom on iOS */
  }

  button {
    padding: 0.75rem;
  }
}

.link {
  color: var(--color-primary);
  text-decoration: none;
  font-size: 1rem;
  font-weight: 500;
}

.link:hover {
  text-decoration: underline;
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
</style>
