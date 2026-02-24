<template>
  <div class="config-section">
    <h2>Sync Delay Settings</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="fronter-channel-wait-increment">Minimum Sync Delay</label>
        <p class="config-description">
          Whenever switches occur, PluralSync will wait at least this delay before pushing the sync
          to the other platforms. This can be useful if you want to register multiple switches in a
          short duration and want to avoid small short-duration switches on the platforms (such as
          PluralKit).
          <br />
          The more switches are registered within a few minutes, the more delayed the update will be
          to avoid uncessary intermediate updates - and to give you more time to register your
          switches. Hence, if you configure this to 60 seconds and do many switches in short
          duration, then PluralSync will wait longer than those 60s (but never more than 3 minutes).
          <br />
          Duration can be between 100ms and 3min. Default: 100ms.
        </p>
        <div class="delay-input-group">
          <input
            id="fronter-channel-wait-increment"
            type="number"
            v-model.number="displayValue"
            :placeholder="displayPlaceholder"
          />
          <select id="fronter-channel-wait-unit" v-model="selectedUnit" class="unit-select">
            <option value="ms">milliseconds</option>
            <option value="s">seconds</option>
            <option value="min">minutes</option>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { UserConfigDbEntries } from '@/pluralsync.bindings'

interface Props {
  config: UserConfigDbEntries
  defaults: UserConfigDbEntries
}

const props = defineProps<Props>()

type TimeUnit = 'ms' | 's' | 'min'

const selectedUnit = ref<TimeUnit>('ms')

const displayValue = ref<number>(100)

const displayPlaceholder = computed(() => {
  const defaultValue = props.defaults.fronter_channel_wait_increment ?? 100
  console.log('displayPlaceholder: defaultValue', defaultValue)
  if (selectedUnit.value === 'ms') return defaultValue.toString()
  if (selectedUnit.value === 's') return (defaultValue / 1000).toString()
  return (defaultValue / 60000).toString()
})

const selectBestUnit = (ms: number) => {
  console.log('selectBestUnit', ms)
  // Pick the smallest unit where the value is a whole number
  if (ms >= 60000 && ms % 60000 === 0) {
    selectedUnit.value = 'min'
    displayValue.value = ms / 60000
  } else if (ms >= 1000 && ms % 1000 === 0) {
    selectedUnit.value = 's'
    displayValue.value = ms / 1000
  } else {
    selectedUnit.value = 'ms'
    displayValue.value = ms
  }
}

function saveNewValue() {
  let msValue = displayValue.value
  if (selectedUnit.value === 'ms') {
    msValue = msValue
  } else if (selectedUnit.value === 's') {
    msValue = msValue * 1000
  } else {
    msValue = msValue * 60000
  }
  console.log('watch: msValue', msValue)
  props.config.fronter_channel_wait_increment = msValue
}

// when the value or unit changes, recalculate the value for saving
watch(displayValue, saveNewValue)
watch(selectedUnit, saveNewValue)

// only run once during initialization, we select the best unit for display
watch(
  () => props.config.fronter_channel_wait_increment,
  (newValue) => {
    if (newValue !== undefined) {
      selectBestUnit(newValue)
    }
  },
  { once: true },
)
</script>

<style scoped>
@import url('../assets/config.css');

.delay-input-group {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.delay-input-group input {
  flex: 1;
  min-width: 120px;
}

.unit-select {
  padding: 0.5rem;
  border: 1px solid #ccc;
  border-radius: 4px;
  background-color: #fff;
  font-size: 0.9rem;
  min-width: 120px;
}
</style>
