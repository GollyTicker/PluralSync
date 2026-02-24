<template>
  <div class="config-section">
    <h2>Fronting History Retention</h2>
    <div class="config-grid">
      <div class="config-item">
        <p class="config-description">
          History is currently
          <strong>{{ historyEnabled ? 'enabled' : 'disabled' }}</strong
          >.
          <span v-if="!historyEnabled"
            >Set both limit and days to values greater than 0 to enable history retention.</span
          >
        </p>
      </div>
      <div class="config-item">
        <label for="history-limit">History Entries Count</label>
        <p class="config-description">
          Maximum number of fronting history entries to keep for display on the history section of
          PluralSync. Set to 0 to disable history retention. Range: 0 to 1000.
        </p>
        <input
          id="history-limit"
          type="number"
          min="0"
          max="1000"
          v-model.number="config.history_limit"
          :placeholder="defaults.history_limit?.toString()"
        />
      </div>
      <div class="config-item">
        <label for="history-days">History Retention Days</label>
        <p class="config-description">
          Maximum age of history entries in days. Entries older than this will be automatically
          deleted. Set to 0 disable history retention. Range: 0 to 30.
        </p>
        <input
          id="history-days"
          type="number"
          min="0"
          max="30"
          v-model.number="config.history_truncate_after_days"
          :placeholder="defaults.history_truncate_after_days?.toString()"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { UserConfigDbEntries } from '@/pluralsync.bindings'

interface Props {
  config: UserConfigDbEntries
  defaults: UserConfigDbEntries
}

const props = defineProps<Props>()

const historyEnabled = computed(() => {
  const limit = props.config.history_limit ?? props.defaults.history_limit!
  const days =
    props.config.history_truncate_after_days ?? props.defaults.history_truncate_after_days!
  return limit > 0 && days > 0
})
</script>

<style scoped>
@import url('../assets/config.css');
</style>
