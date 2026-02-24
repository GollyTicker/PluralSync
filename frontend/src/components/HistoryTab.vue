<template>
  <div class="history-container">
    <h1>Fronting History</h1>
    <p class="history-description">
      View your recent fronting status changes. This history helps you track when switches occurred
      and see the status text at that time. History retention can be configured and disabled/enabled
      in the settings.
    </p>
    <div v-if="history.length === 0" class="history-empty">
      <p>No history entries found.</p>
      <p class="history-hint">
        Fronting history retention is either disabled or no fronting changes have been recorded yet.
      </p>
    </div>
    <div v-else class="history-list">
      <div v-for="entry in history" :key="entry.id" class="history-item">
        <div class="history-header">
          <span class="history-status-text">{{ entry.status_text }}</span>
          <span class="history-timestamp">
            <span>{{ formatRelativeTime(entry.created_at) }}</span>
            <span class="history-timezone">{{ new Date(entry.created_at).toLocaleString() }}</span>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import type { HistoryEntry } from '@/pluralsync.bindings'
import { pluralsync_api } from '@/pluralsync_api'

const history = ref<HistoryEntry[]>([])

let refreshInterval: number | undefined = undefined

function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSecs = Math.floor(diffMs / 1000)
  const diffMins = Math.floor(diffSecs / 60)
  const diffHours = Math.floor(diffMins / 60)
  const diffDays = Math.floor(diffHours / 24)

  if (diffSecs < 60) {
    return 'just now'
  } else if (diffMins < 60) {
    return `${diffMins} min${diffMins !== 1 ? 's' : ''} ago`
  } else if (diffHours < 24) {
    return `${diffHours} hour${diffHours !== 1 ? 's' : ''} ago`
  } else {
    return `${diffDays} day${diffDays !== 1 ? 's' : ''} ago`
  }
}

const fetchHistory = async () => {
  try {
    history.value = await pluralsync_api.get_history_fronting()
    console.log('get_history_fronting: ', history.value)
  } catch (e) {
    console.warn(e)
  }
}

onMounted(async () => {
  await fetchHistory()
  refreshInterval = setInterval(fetchHistory, 10000) // Refresh every 10 seconds
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})
</script>

<style scoped>
@import url('../assets/status.css');
</style>
