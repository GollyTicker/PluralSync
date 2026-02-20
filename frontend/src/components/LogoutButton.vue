<template>
  <h1>Logging out...</h1>
</template>

<script setup lang="ts">
import { logoutAndBackToStart } from '@/jwt'
import { onMounted } from 'vue'

onMounted(() => {
  logoutAndBackToStart()
})
</script>
