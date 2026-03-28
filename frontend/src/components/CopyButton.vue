<template>
  <button type="button" @click="copy" class="copy-button" :title="label">
    {{ label }}
  </button>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  text?: string
}>()

const label = ref('Copy')

async function copy() {
  try {
    if (props.text === undefined) return
    await navigator.clipboard.writeText(props.text)
    label.value = 'Copied!'
    setTimeout(() => {
      label.value = 'Copy'
    }, 2000)
  } catch (err) {
    console.log(err)
    label.value = 'Failed'
    setTimeout(() => {
      label.value = 'Copy'
    }, 2000)
  }
}
</script>

<style scoped>
.copy-button {
  padding: 0.25rem 0.5rem;
  background: var(--color-primary);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.85em;
  margin: 0px;
}

.copy-button:hover {
  background: var(--color-secondary);
  color: white;
}
</style>
