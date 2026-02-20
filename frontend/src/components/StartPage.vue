<template>
  <div>
    <h1>Welcome</h1>
  </div>
</template>

<script setup lang="ts">
import router from '@/router'
import { onMounted } from 'vue'

onMounted(() => {
  router.push('/login')
})
</script>
