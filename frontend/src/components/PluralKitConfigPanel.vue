<template>
  <div class="config-section">
    <h2>PluralKit</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="enable_to_pluralkit">Enable Sync to PluralKit</label>
        <p class="config-description">
          Automatically synchronize your fronting status to PluralKit.
          <br />
          Note: Syncing of the member information itself from SimplyPlural to PluralKit is not done
          automatically yet. You need to import/export your system between both manually for now to
          ensure, that all the members exist in both places.
          <br />
          We simply tell PluralKit the member IDs of the fronters (after they have been filtered
          through the above privacy conditions and IF the pluralkit id is defined in SimplyPlural).
          The members themselves are shown with the same privacy rules as you have configured them
          in PluralKit.
          <br />
          PluralKit cares about the order of the fronters (at it's relevant for autoproxy) but
          SimplyPlural does not. Hence, when we sync to pluralkit, we don't change the order of
          existing fronters in pluralkit. We only remove fronters which are not fronting anymore
          according to SimplyPlural - and add new fronters at the end of the list.
        </p>
        <input id="enable_to_pluralkit" type="checkbox" v-model="config.enable_to_pluralkit" />
      </div>
      <div class="config-item">
        <label for="pluralkit_token">PluralKit Token</label>
        <p class="config-description">
          The token to authenticate with the PluralKit API. You can get this from the PluralKit bot
          via "pk;token".
        </p>
        <input
          id="pluralkit_token"
          type="password"
          :value="config.pluralkit_token?.secret"
          @input="setSecret('pluralkit_token', $event)"
        />
      </div>
    </div>
  </div>
</template>

<!-- eslint-disable vue/no-mutating-props -->
<script setup lang="ts">
import type { UserConfigDbEntries, Decrypted } from '@/pluralsync.bindings'

interface Props {
  config: UserConfigDbEntries
}

const props = defineProps<Props>()

type SecretKeys = 'pluralkit_token'

function setSecret(key: SecretKeys, event: Event) {
  const target = event.target as HTMLInputElement
  if (target.value !== '') {
    props.config[key] = <Decrypted>{ secret: target.value }
  } else {
    props.config[key] = undefined
  }
}
</script>
