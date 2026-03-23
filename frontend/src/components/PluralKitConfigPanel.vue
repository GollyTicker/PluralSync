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
      <div class="config-item">
        <label for="enable_from_pluralkit">Enable Sync from PluralKit</label>
        <p class="config-description">
          PluralSync will listen for changes in your system and fronting from PluralKit via webhook
          and update your status on all connected platforms. This option cannot be simultanously
          activated, when pluralkit is also a sync destination.
        </p>
        <input id="enable_from_pluralkit" type="checkbox" v-model="config.enable_from_pluralkit" />
      </div>
      <div class="config-item">
        <label>PluralKit Webhook Setup</label>
        <div class="copyable-field">
          <input
            type="text"
            :value="pkWebhookSetupCommand"
            readonly
            class="webhook-command-input"
          />
          <CopyButton :text="pkWebhookSetupCommand" />
        </div>
      </div>
      <p class="config-description">Follow these steps to set up the webhook:</p>
      <ol class="config-description" style="padding-left: 1.5rem">
        <li>Click copy the PluralKit Webhook setup command beow.</li>
        <li>Open a Discord DM to the PluralKit bot and paste and send the command to PluralKit.</li>
        <li>PluralKit will respond with a signing token. Copy that token.</li>
        <li>Paste the signing token into the field below.</li>
        <li>Confirm the signing token in the DM with PluralKit</li>
        <li>Save your changes in PluralSync</li>
      </ol>
      <div v-if="config.enable_from_pluralkit" class="config-item">
        <label for="from_pluralkit_webhook_signing_token">PluralKit Webhook Signing Token</label>
        <p class="config-description">
          The signing token provided by PluralKit when registering the webhook. This is used to
          verify that incoming webhook requests are authentic.
        </p>
        <input
          id="from_pluralkit_webhook_signing_token"
          type="password"
          :value="config.from_pluralkit_webhook_signing_token?.secret"
          @input="setSecret('from_pluralkit_webhook_signing_token', $event)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { UserConfigDbEntries, Decrypted, UserId } from '@/pluralsync.bindings'
import CopyButton from '@/components/CopyButton.vue'

interface Props {
  config: UserConfigDbEntries
  userId?: UserId // for webhook URL construction
}

const props = defineProps<Props>()

type SecretKeys = 'pluralkit_token' | 'from_pluralkit_webhook_signing_token'

function setSecret(key: SecretKeys, event: Event) {
  const target = event.target as HTMLInputElement
  if (target.value !== '') {
    props.config[key] = <Decrypted>{ secret: target.value }
  } else {
    props.config[key] = undefined
  }
}

// Construct webhook URL
const pkWebhookSetupCommand = computed(() => {
  const baseUrl = window.location.origin
  return `pk;system webhook ${baseUrl}/api/webhook/pluralkit/${props.userId?.inner}`
})
</script>

<style scoped>
@import url('../assets/config.css');

.copyable-field {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.copyable-field input {
  flex: 1;
}

.webhook-command-input {
  padding: 0.5rem;
  border: 1px solid black;
  border-radius: 4px;
  font-family: monospace;
  font-size: 0.6em;
  background-color: lightgrey;
}
</style>
