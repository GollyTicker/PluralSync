<template>
  <div class="config-container">
    <h1>Settings</h1>
    <p>
      Configure the various updaters to synchronize your fronting status. Whenever you enter a
      switch into SimplyPlural the fronting will be updated on all configured and enabled platforms
      within seconds!
      <br />
      At any point, you can remove your login information from here by disabling the corresponding
      updater, emptying the field and saving the changes.
      <br />
      You can change your password by clicking on "Forgot Password?" on the login page.
      <br />
      Stuck with something and still not working despite having re-read the info again and looked
      exactly? Join the discord server and feel free to ask the community for help. (Link in the
      footer of this website)
    </p>
    <form @submit.prevent="saveConfigAndRestart" autocomplete="off">
      <div class="config-section">
        <h2>PluralSync Settings</h2>
      </div>
      <button type="submit">Save and Restart</button>
      <p class="config-update-status">{{ status }}</p>
      <SyncConfigPanel :config="config" :defaults="defaults" />
      <SimplyPluralConfigPanel :config="config" />
      <PluralKitConfigPanel :config="config" />
      <WebsiteConfigPanel :config="config" :defaults="defaults" />
      <FrontingStatusTextPanel :config="config" :defaults="defaults" />
      <DiscordConfigPanel :config="config" />
      <VRChatConfigPanel :config="config" />
      <HistoryConfigPanel :config="config" :defaults="defaults" />
      <button type="submit">Save and Restart</button>
      <p class="config-update-status">{{ status }}</p>
      <div class="config-section">
        <h2 style="color: #d32f2f">Account Settings</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="current-email">Current Email</label>
            <p class="config-description">
              Your current email address associated with your PluralSync account.
            </p>
            <input id="current-email" type="email" :value="currentEmail" disabled />
          </div>
          <div class="config-item">
            <label for="new-email">Change Email Address</label>
            <p class="config-description">
              Enter your new email address. You will receive a confirmation link at the new email
              address to complete the change.
            </p>
            <input
              id="new-email"
              type="email"
              v-model="newEmail"
              placeholder="Enter new email address"
              autocomplete="off"
            />
            <button
              @click.prevent="requestEmailChange"
              type="button"
              id="email-change-button"
              :disabled="emailChangeLoading"
            >
              {{ emailChangeLoading ? 'Sending...' : 'Change' }}
            </button>
            <p id="email-change-status">{{ emailChangeStatus }}</p>
          </div>
          <div
            class="config-item"
            style="border-top: 2px solid #d32f2f; padding-top: 1rem; margin-top: 1rem"
          >
            <label for="delete-account-button" style="color: #d32f2f">Delete Account</label>
            <p class="config-description">
              Permanently delete your PluralSync account and all associated data.
            </p>
            <button
              @click="navigateToDeleteAccount"
              type="button"
              id="delete-account-button"
              class="danger-button"
            >
              Delete Account
            </button>
          </div>
        </div>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, type Ref } from 'vue'
import router from '@/router'
import type { UserConfigDbEntries } from '@/pluralsync.bindings'
import { detailed_error_string, pluralsync_api } from '@/pluralsync_api'
import SimplyPluralConfigPanel from '@/components/SimplyPluralConfigPanel.vue'
import PluralKitConfigPanel from '@/components/PluralKitConfigPanel.vue'
import WebsiteConfigPanel from '@/components/WebsiteConfigPanel.vue'
import FrontingStatusTextPanel from '@/components/FrontingStatusTextPanel.vue'
import DiscordConfigPanel from '@/components/DiscordConfigPanel.vue'
import VRChatConfigPanel from '@/components/VRChatConfigPanel.vue'
import HistoryConfigPanel from '@/components/HistoryConfigPanel.vue'
import SyncConfigPanel from '@/components/SyncConfigPanel.vue'

const config: Ref<UserConfigDbEntries> = ref({} as UserConfigDbEntries)
const defaults: Ref<UserConfigDbEntries> = ref({} as UserConfigDbEntries)
const status = ref('')
const currentEmail = ref('')
const newEmail = ref('')
const emailChangeStatus = ref('')
const emailChangeLoading = ref(false)

async function fetchConfig() {
  try {
    config.value = await pluralsync_api.get_config()
    console.log('Received user config: ', config.value)
  } catch (e) {
    console.warn(e)
  }
}

async function fetchUserInfo() {
  try {
    const userInfo = await pluralsync_api.get_user_info()
    currentEmail.value = userInfo.email?.inner || ''
    console.log('Received user info: ', userInfo)
  } catch (e) {
    console.warn(e)
  }
}

async function fetchDefaults() {
  try {
    defaults.value = await pluralsync_api.get_defaults()
    console.log('Received default config: ', defaults.value)
  } catch (e) {
    console.warn(e)
  }
}

async function saveConfigAndRestart() {
  try {
    // Ensure that empty strings are interpreted as undefined.
    // Vue unfortunately breaks type-safety, because v-model.number returns number as a type
    // but allows invalid strings at runtime and simply returns them unchanged.
    for (const key in config.value) {
      if (config.value[key as keyof UserConfigDbEntries] === '') {
        console.log('before save: setting key ' + key + ' to undefined.')
        config.value[key as keyof UserConfigDbEntries] = undefined
      }
    }

    await pluralsync_api.set_config_and_restart(config.value)
    status.value = 'Config saved successfully and restarted updaters!'
  } catch (err) {
    console.warn(err)
    status.value =
      'Failed to save config and restart updaters. Error: ' + detailed_error_string(err)
  }
}

async function requestEmailChange() {
  if (!newEmail.value || !newEmail.value.trim()) {
    emailChangeStatus.value = 'Please enter a new email address.'
    return
  }

  if (newEmail.value === currentEmail.value) {
    emailChangeStatus.value = 'New email must be different from your current email.'
    return
  }

  emailChangeLoading.value = true
  emailChangeStatus.value = ''
  try {
    await pluralsync_api.requestEmailChange({ new_email: { inner: newEmail.value } })
    emailChangeStatus.value =
      'Confirmation link sent! Check your new email address to verify the change.'
    newEmail.value = ''
  } catch (err) {
    console.warn(err)
    emailChangeStatus.value = 'Failed to request email change. Error: ' + detailed_error_string(err)
  } finally {
    emailChangeLoading.value = false
  }
}

function navigateToDeleteAccount() {
  router.push('/settings/delete-account')
}

onMounted(async () => {
  await fetchConfig()
  await fetchUserInfo()
  await fetchDefaults()
})
</script>

<style scoped>
@import url('../assets/config.css');
</style>
