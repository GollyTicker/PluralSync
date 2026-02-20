<template>
  <div class="config-section">
    <h2>Fronting Status Text</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="status_prefix">Fronting Text Prefix</label>
        <p class="config-description">
          What to begin the fronting status message with. E.g. the "F: " in the example "F: Ania,
          Björn, Claire".
        </p>
        <input
          id="status_prefix"
          type="text"
          v-model="config.status_prefix"
          :placeholder="defaults.status_prefix"
        />
      </div>
      <div class="config-item">
        <label for="status_no_fronts">Fronting Text When 0 Fronters</label>
        <p class="config-description">What to show, when no fronters are currently active.</p>
        <input
          id="status_no_fronts"
          type="text"
          v-model="config.status_no_fronts"
          :placeholder="defaults.status_no_fronts"
        />
      </div>
      <div class="config-item">
        <label for="status_truncate_names_to">Name Shortening</label>
        <p class="config-description">
          The platforms have limits on how long the status message can be. If the fronting status
          would be too long for the platform (due to fronters with long names or due to a many
          simultanous fronters), then PluralSync will shorten the fronters names to a specific
          length to fit the length. E.g. "Claire" would become "Cla" if this is set to "3". If the
          shorted version is still to long, then PluralSync will simply show the number of fronters.
        </p>
        <input
          id="status_truncate_names_to"
          type="number"
          v-model.number="config.status_truncate_names_to"
          :placeholder="defaults.status_truncate_names_to?.toString()"
        />
      </div>
    </div>
  </div>
</template>

<!-- eslint-disable vue/no-mutating-props -->
<script setup lang="ts">
import type { UserConfigDbEntries } from '@/pluralsync.bindings'

interface Props {
  config: UserConfigDbEntries
  defaults: UserConfigDbEntries
}

defineProps<Props>()
</script>

<style scoped>
@import url('../assets/config.css');
</style>
