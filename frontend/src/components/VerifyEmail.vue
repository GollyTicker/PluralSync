<template>
  <div class="verify-email-container">
    <h1>Email Verification</h1>
    <div v-if="loading" class="status-box">
      <p class="status-message">Verifying your email address...</p>
    </div>
    <div v-else-if="error" class="status-box">
      <p class="status-message error-message">{{ error }}</p>
      <p class="back-link">
        <router-link to="/">Back to home</router-link>
      </p>
    </div>
    <div v-else-if="success" class="status-box">
      <p class="status-message success-message">{{ message }}</p>
      <p class="back-link">
        <router-link :to="successRedirectPath">{{ successRedirectLabel }}</router-link>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { pluralsync_api, detailed_error_string } from '@/pluralsync_api'

const route = useRoute()
const loading = ref(true)
const success = ref(false)
const error = ref<string | undefined>(undefined)
const message = ref('')
const successRedirectPath = ref('/login')
const successRedirectLabel = ref('Log in')

onMounted(async () => {
  const token = route.query.token as string | undefined
  if (!token) {
    error.value = 'Invalid or missing verification token.'
    loading.value = false
    return
  }

  try {
    const response = await pluralsync_api.verifyEmail({
      inner: { inner: token },
    })
    message.value = response.message
    success.value = true
    successRedirectPath.value = '/login'
    successRedirectLabel.value = 'Log in'
  } catch (err: any) {
    error.value = detailed_error_string(err)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
@import url('../assets/message.css');

.verify-email-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 80vh;
  padding: 2rem;
}

h1 {
  text-align: center;
  margin-bottom: 1.5rem;
  color: var(--color-heading);
}

.status-box {
  background: var(--vt-c-white);
  padding: 2rem;
  border-radius: 8px;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
  width: 100%;
  max-width: 400px;
  text-align: center;
}

.status-message {
  color: var(--color-text);
  margin-bottom: 1rem;
  line-height: 1.5;
}

.back-link {
  margin-top: 1.5rem;
  font-size: 14px;
}

.back-link a {
  color: var(--color-primary);
  text-decoration: none;
  padding: 0.5rem 1rem;
  display: inline-block;
  border-radius: 4px;
  transition: background-color 0.3s ease;
}

.back-link a:hover {
  background-color: var(--color-primary);
  color: var(--background-white);
}
</style>
