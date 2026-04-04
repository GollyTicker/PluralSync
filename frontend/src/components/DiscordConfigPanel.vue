<template>
  <div>
    <div class="config-section">
      <h2>Discord via Bridge</h2>
      <div class="config-grid">
        <div class="config-item">
          <label for="enable_discord">Enable Discord Rich Presence</label>
          <input id="enable_discord" type="checkbox" v-model="config.enable_discord" />
        </div>
        <div class="config-item">
          <label for="discord_rich_presence_url">Rich Presence Link</label>
          <p class="config-description">
            Choose what happens when someone clicks on your Discord Rich Presence.
          </p>
          <select id="discord_rich_presence_url" v-model="config.discord_rich_presence_url">
            <option value="PluralSyncFrontingWebsiteIfDefined">
              Your Systems' PluralSync Fronting Website (if configured; default)
            </option>
            <option value="CustomUrl">A Custom URL</option>
            <option value="PluralSyncAboutPage">PluralSync About Page</option>
            <option value="None">Nothing</option>
          </select>
        </div>
        <div v-if="config.discord_rich_presence_url === 'CustomUrl'" class="config-item">
          <label for="discord_rich_presence_url_custom">Rich Presence Custom URL</label>
          <input
            id="discord_rich_presence_url_custom"
            type="url"
            v-model="config.discord_rich_presence_url_custom"
            placeholder="https://example.com"
          />
        </div>
        <div class="config-item">
          <p class="config-description">
            If enabled, shows your fronting status as a
            <a href="https://discord.com/developers/docs/rich-presence/overview"
              >Rich Presence on Discord</a
            >.
            <br />
            This option only works via the PluralSync-Bridge, which you need to run on the same
            computer as your discord. For that, open
            <a target="_blank" :href="PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL">this</a>, then open
            the first "Assets" section to see and download the "PluralSync.Bridge" for your
            platform.
            <br />
            Then start it on the computer where Discord Desktop is running. You might get a warning,
            that the executable is not signed or executable. Simply accept warning that and run it.
            (For small projects, it's infeasible to get this signed.)
            <br />
            Once started, you can login to PluralSync. (You can safely ignore the "Variant" field.)
            When you have discord running on the same computer, PluralSync will show itself as a
            rich presence activity and display the fronting status from there.
            <br />
            You may need to enable Rich Presence in Discord under the "Activity Privacy" settings.
            <br />
            <img
              src="/discord_rich_presence_activate.png"
              alt="Discord Activity Privacy Settings"
            />
            <br />
            The benefit of this method, is that it is Discord ToS compliant. The drawback of this is
            that these updates only work as long as your PluralSync bridge and Discord are running
            locally.
            <br />
            The PluralSync bridge automatically updates on <b>some</b> platforms. Check the text in
            the bridge to see, if an update is available and how to update.
            <i>The bridge must be up-to-date with the server for it work.</i>
            If you want to minimize the bridge, you can do so via the program
            <a href="https://rbtray.sourceforge.net/">RBTray</a> on Windows. (SystemTray is buggy on
            Windows which is why qwe suggest this program instead.)
          </p>
        </div>
      </div>
    </div>
    <div class="config-section">
      <h2>Discord via Token ⚠️</h2>
      <div class="config-grid">
        <div class="config-item">
          <label for="enable_discord_status_message">Enable Discord Status Message ⚠️</label>
          <input
            id="enable_discord_status_message"
            type="checkbox"
            v-model="config.enable_discord_status_message"
          />
          <p class="config-description">
            You can also directly set the custom status on your discord account.
            <br />
            For that, PluralSync will need a discord token. PluralSync will update the discord
            status for you regularly.
            <br />
            <span class="warning"
              >WARNING! This violates Discord Terms of Service. Use at your own risk! This option
              might be removed at any point!</span
            >
            <br />
            This method produces a more visible fronting status, but isn't as clean ToS-compliant as
            the previous option. (Because Discord may remove this at any point.)
          </p>
        </div>
        <div class="config-item">
          <label for="discord_status_message_token">Discord Status Message Token ⚠️</label>
          <input
            id="discord_status_message_token"
            type="password"
            :value="config.discord_status_message_token?.secret"
            @input="setSecret('discord_status_message_token', $event)"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UserConfigDbEntries, Decrypted } from '@/pluralsync.bindings'
import { PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL } from '@/pluralsync.bindings'

interface Props {
  config: UserConfigDbEntries
}

const props = defineProps<Props>()

type SecretKeys = 'discord_status_message_token'

function setSecret(key: SecretKeys, event: Event) {
  const target = event.target as HTMLInputElement
  if (target.value !== '') {
    props.config[key] = <Decrypted>{ secret: target.value }
  } else {
    props.config[key] = undefined
  }
}
</script>

<style scoped>
@import url('../assets/config.css');
</style>
