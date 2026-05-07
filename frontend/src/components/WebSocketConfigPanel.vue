<template>
  <div v-if="websocketAvailable" class="config-section">
    <h2>WebSocket Push Source</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="enable_from_websocket">Enable Sync from WebSocket</label>
        <p class="config-description">
          Allow external clients to push fronting status updates via the WebSocket endpoint at
          <code>/api/user/platform/pluralsync/events</code>. This enables custom integrations (e.g.,
          a custom fronting tracker or third-party system) to feed fronting data into PluralSync's
          synchronization pipeline.
          <br />
          Note, that only one system manager for fronting is supported at a time (either
          SimplyPlural, PluralKit, or WebSocket as source).
        </p>
        <input id="enable_from_websocket" type="checkbox" v-model="config.enable_from_websocket" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UserConfigDbEntries } from '@/pluralsync.bindings'
import { ref, type Ref } from 'vue'

interface Props {
  config: UserConfigDbEntries
}

const props = defineProps<Props>()

// NOTE: hidden feature. only enabled on my private instances currently. will be made public with an unstable API in future
const websocketAvailable: Ref<boolean> = ref(
  location.href.includes('https://private.pluralsync') ||
    location.href.includes('https://dev-online.pluralsync'),
)
</script>

<style scoped>
@import url('../assets/config.css');

code {
  background-color: #f0f0f0;
  padding: 0.1em 0.3em;
  border-radius: 3px;
  font-family: monospace;
  font-size: 0.9em;
}
</style>
