<template>
  <div class="status-container">
    <h1 id="status-page-title">Updaters Status</h1>
    <p v-if="fronting_status?.inner">
      Example fronting status:
      <span id="fronting-status-example" class="fronting-status-text">{{ fronting_status?.inner }}</span>
    </p>
    <div class="status-list">
      <div v-for="(status, name) in updaters" :key="name" class="status-item">
        <span class="service-name">{{ name }}</span>
        <span :id="name + '-status'" :class="['status-badge', 'status-' + statusKind(status!)]">{{
          statusKind(status!)
        }}</span>
        <span class="status-info">{{ statusInfo(status!) }}</span>
      </div>
      <div v-if="Object.keys(updaters).length === 0" class="status-item">
        <span class="status-info">
          No updaters configured. Go to <a href="/config">Settings</a> to add some!
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, type Ref } from 'vue'
import type {
  GenericFrontingStatus,
  UpdaterStatus,
  UserUpdatersStatuses,
} from '@/pluralsync.bindings'
import { pluralsync_api } from '@/pluralsync_api'

const updaters: Ref<UserUpdatersStatuses> = ref({})
const fronting_status: Ref<GenericFrontingStatus | undefined> = ref(undefined)

let refreshViewIntervalTimer1: number | undefined = undefined
let refreshViewIntervalTimer2: number | undefined = undefined

function statusKind(status: UpdaterStatus): string {
  switch (status) {
    case 'Disabled':
      return status
    case 'Running':
      return status
    case 'Starting':
      return status
    default:
      return 'Error' // note. how to make this as future-proof as rust matches?
  }
}

function statusInfo(status: UpdaterStatus): string {
  switch (status) {
    case 'Disabled':
      return ''
    case 'Running':
      return ''
    case 'Starting':
      return ''
    default:
      return status.Error // note. how to make this as future-proof as rust matches?
  }
}

const fetchUpdatersState = async () => {
  try {
    updaters.value = await pluralsync_api.get_updater_status()
    console.log('get_updater_status: ', updaters.value)
  } catch (e) {
    console.warn(e)
  }
}

const fetchFrontingStatus = async () => {
  try {
    fronting_status.value = await pluralsync_api.get_fronting_status()
    console.log('get_fronting_status: ', fronting_status.value)
  } catch (e) {
    console.warn(e)
  }
}

onMounted(async () => {
  await fetchUpdatersState()
  await fetchFrontingStatus()
  refreshViewIntervalTimer1 = setInterval(fetchUpdatersState, 5000)
  refreshViewIntervalTimer2 = setInterval(fetchFrontingStatus, 5000)
})

onUnmounted(() => {
  clearInterval(refreshViewIntervalTimer1)
  clearInterval(refreshViewIntervalTimer2)
})
</script>

<style scoped>
@import url('../assets/status.css');

.service-name {
  font-weight: bold;
}

.status-info {
  margin-left: 1rem;
}
</style>
