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
    </p>
    <form @submit.prevent="saveConfigAndRestart" autocomplete="off">
      <button type="submit">Save and Restart</button>
      <p id="config-update-status">{{ status }}</p>
      <div class="config-section">
        <h2>Simply Plural</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="simply_plural_token">Simply Plural Token</label>
            <p class="config-description">
              The private READ-token used by PluralSync to access your Simply Plural system to check
              for changes. To make one, open
              <a href="https://app.apparyllis.com/" target="_blank">SimplyPlural</a>, go to Settings
              > Account > Tokens, create a READ token and copy-paste it here.
            </p>
            <input
              id="simply_plural_token"
              type="password"
              :value="config.simply_plural_token?.secret"
              @input="setSecret('simply_plural_token', $event)"
            />
          </div>
          <div class="config-item">
            <p class="config-description">
              The following toggles and settings allow you to configure the privacy and visibility
              of the members and custom fronts. The "Show ..." toggles are used to show/hide
              categories of fronts. If they're OFF, then nothing of that category is shown.
              <br />
              Each fronter must pass all conditions detailed below to be shown. E.g. if a fronter is
              a non-archived member with the setting "Prevent notifications on front change" enabled
              in SimplyPlural, then member will be shown exactly when (1) the member is fronting AND
              (2) "Show Active Members" is ON AND (3) Respect "Prevent notifications on front
              change" is OFF. If any of the above conditions are not met, then the member is not
              shown.
            </p>
            <p class="warning">
              {{
                !config.show_members_non_archived &&
                !config.show_members_archived &&
                !config.show_custom_fronts
                  ? "Nothing will be shown, since all 'Show' toggles are OFF."
                  : ''
              }}
            </p>
            <label for="show_members_non_archived"> Show Active Members </label>
            <p class="config-description">
              Show members which are <span style="font-weight: bold">not archived</span>. They might
              still be hidden, if the other conditions make them hidden. Recommended to enable.
            </p>
            <input
              id="show_members_non_archived"
              type="checkbox"
              v-model="config.show_members_non_archived"
            />
          </div>
          <div class="config-item">
            <label for="show_members_archived"> Show Archived Members </label>
            <p class="config-description">
              Show <span style="font-weight: bold">archived</span> members. They might still be
              hidden, if the other conditions make them hidden.
            </p>
            <input
              id="show_members_archived"
              type="checkbox"
              v-model="config.show_members_archived"
            />
          </div>
          <div class="config-item">
            <label for="respect_front_notifications_disabled">
              Respect "Prevent notifications on front change"
            </label>
            <p class="config-description">
              If ON, then the member will be hidden, if their fronting change is configured not
              notify others. If OFF, then this setting in Simply Plural is ignored.
            </p>
            <input
              id="respect_front_notifications_disabled"
              type="checkbox"
              v-model="config.respect_front_notifications_disabled"
            />
          </div>
          <div class="config-item">
            <label for="show_custom_fronts">Show Custom Fronts</label>
            <input id="show_custom_fronts" type="checkbox" v-model="config.show_custom_fronts" />
          </div>
          <div class="config-item">
            <label for="privacy_fine_grained"> Fine-Grained Control using Privacy Buckets </label>
            <p class="config-description">
              You can optionally use Simply Plural "Privacy Buckets" to manage the visibility of
              fronters on a more deailed level. You can use one of these options:
            </p>
            <ol class="config-description">
              <li>Not use privacy buckets at all and only use the above "Show" toggles</li>
              <li>
                Add the
                <span
                  style="font-weight: bold"
                  class="copyable"
                  @click="copyText('PluralSync', $event)"
                  title="Click to copy"
                  >PluralSync</span
                >
                user as a friend on Simply Plural and assign that friend to your existing privacy
                buckets. PluralSync will then show any fronters which are are in privacy buckets the
                PluralSync friend is assigned to. The above "Show" toggles still apply. (Note, that
                the privacy settings you can configure for friends like "They can see your shared
                members" etc are IGNORED. Only the privacy buckets are used.)
              </li>
              <li>
                Directly choose the privacy buckets on this PluralSync Website here and any fronts
                assigned to the privacy buckets selected here will be shown. The above "Show"
                toggles still apply.
              </li>
            </ol>
            <p></p>
            <select v-model="config.privacy_fine_grained">
              <option value="NoFineGrained">no fine grained control (default)</option>
              <option value="ViaFriend">via PluralSync-friend on SimplyPlural</option>
              <option value="ViaPrivacyBuckets">via privacy buckets configured below</option>
            </select>
          </div>
          <div class="config-item">
            <label for="config.privacy_fine_grained_buckets"></label>
            <p class="config-description">
              If you choose "via privacy buckets" above, then you can configure which privacy
              buckets to use here. You can chose multiple privacy buckets.

              {{ privacyBucketsStatus }}
            </p>
            <select
              v-model="config.privacy_fine_grained_buckets"
              multiple
              :disabled="config.privacy_fine_grained !== 'ViaPrivacyBuckets'"
            >
              <option
                v-for="bucket in simply_plural_privacy_buckets"
                :key="bucket.id"
                :value="bucket.id"
              >
                {{ bucket.name }}
              </option>
            </select>
          </div>
        </div>
      </div>
      <div class="config-section">
        <h2>PluralKit</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="enable_to_pluralkit">Enable Sync to PluralKit</label>
            <p class="config-description">
              Automatically synchronize your fronting status to PluralKit.
              <br />
              Note: Syncing of the member information itself from SimplyPlural to PluralKit is not
              done automatically yet. You need to import/export your system between both manually
              for now to ensure, that all the members exist in both places.
              <br />
              We simply tell PluralKit the member IDs of the fronters (after they have been filtered
              through the above privacy conditions and IF the pluralkit id is defined in
              SimplyPlural). The members themselves are shown with the same privacy rules as you
              have configured them in PluralKit.
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
              The token to authenticate with the PluralKit API. You can get this from the PluralKit
              bot via "pk;token".
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
      <div class="config-section">
        <h2>Website</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="enable_website">Enable Website</label>
            <p class="config-description">
              Let PluralSync display your fronting status (avatars included) as a webpage. Others
              can simply open the link and see the current fronters without needing to be logged in
              in any of the other platforms.
            </p>
            <input id="enable_website" type="checkbox" v-model="config.enable_website" />
          </div>
          <div class="config-item">
            <label for="website_system_name">System Name</label>
            <p class="config-description">
              The name of your system as the title of the website view.
            </p>
            <input
              id="website_system_name"
              type="text"
              v-model="config.website_system_name"
              :placeholder="defaults.website_system_name"
            />
          </div>
          <div class="config-item">
            <label for="website_url_name">Website Link Part</label>
            <p class="config-description">
              Adapt the link at which your fronting website is shown. For example, if you want your
              link to be "{{ baseUrl }}/fronting/ocean-collective", then set this field to
              "ocean-collective". Once activated, your link will be
              <a :href="baseUrl + '/fronting/' + config.website_url_name" target="_blank">{{
                baseUrl + '/fronting/' + config.website_url_name
              }}</a>
            </p>
            <input
              id="website_url_name"
              type="text"
              v-model="config.website_url_name"
              :placeholder="defaults.website_url_name"
            />
          </div>
        </div>
      </div>
      <div class="config-section">
        <h2>Fronting Status Text</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="status_prefix">Fronting Text Prefix</label>
            <p class="config-description">
              What to begin the fronting status message with. E.g. the "F: " in the example "F:
              Ania, Björn, Claire".
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
              The platforms have limits on how long the status message can be. If the fronting
              status would be too long for the platform (due to fronters with long names or due to a
              many simultanous fronters), then PluralSync will shorten the fronters names to a
              specific length to fit the length. E.g. "Claire" would become "Cla" if this is set to
              "3". If the shorted version is still to long, then PluralSync will simply show the
              number of fronters.
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
      <div class="config-section">
        <h2>Discord via Bridge</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="enable_discord">Enable Discord Rich Presence</label>
            <input id="enable_discord" type="checkbox" v-model="config.enable_discord" />
            <p class="config-description">
              If enabled, shows your fronting status as a
              <a href="https://discord.com/developers/docs/rich-presence/overview"
                >Rich Presence on Discord</a
              >.
              <br />
              This option only works via the PluralSync-Bridge, which you need to run on the same
              computer as your discord. For that, open
              <a target="_blank" :href="PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL">this</a>, then
              open the first "Assets" section to see and download the "PluralSync.Bridge" for your
              platform.
              <br />
              Then start it on the computer where Discord Desktop is running. You might get a
              warning, that the executable is not signed or executable. Simply accept warning that
              and run it. (For small projects, it's infeasible to get this signed.)
              <br />
              Once started, you can login to PluralSync. (You can safely ignore the "Variant"
              field.) When you have discord running on the same computer, PluralSync will show
              itself as a rich presence activity and display the fronting status from there.
              <br />
              You may need to enable Rich Presence in Discord under the "Activity Privacy" settings.
              <br />
              <img
                src="/discord_rich_presence_activate.png"
                alt="Discord Activity Privacy Settings"
              />
              <br />
              The benefit of this method, is that it is Discord ToS compliant. The drawback of this
              is that these updates only work as long as your PluralSync bridge and Discord are
              running locally.
              <br />
              Note, that the PluralSync bridge doesn't automatically update at the moment. If it
              doesn't work then download and install the latest version and try again!
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
              This method produces a more visible fronting status, but isn't as clean ToS-compliant
              as the previous option. (Because Discord may remove this at any point.)
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
      <div class="config-section">
        <h2>VRChat</h2>
        <div class="config-grid">
          <div class="config-item">
            <label for="enable_vrchat">Enable VRChat Status Message ⚠️</label>
            <input id="enable_vrchat" type="checkbox" v-model="config.enable_vrchat" />
            <p class="config-description">
              Shows the fronting status on VRChat in the custom status at your profile in VR and on
              the website.
              <br />
              For that, you will need to login into VRChat such that PluralSync can set the fronting
              status on VRChat's side.
              <br />
              <span class="warning"
                >WARNING! This violates VRChat Terms of Service. Use at your own risk! This option
                might be removed at any point!</span
              >
              <br />
              This method produces a more visible fronting status than using OSC, but isn't as clean
              ToS-compliant.
            </p>
          </div>
          <div class="config-item">
            <label for="vrchat_username">VRChat Username ⚠️</label>
            <input
              id="vrchat_username"
              type="password"
              :value="config.vrchat_username?.secret"
              @input="setSecret('vrchat_username', $event)"
            />
          </div>
          <div class="config-item">
            <label for="vrchat_password">VRChat Password ⚠️</label>
            <input
              id="vrchat_password"
              type="password"
              :value="config.vrchat_password?.secret"
              @input="setSecret('vrchat_password', $event)"
            />
          </div>
          <div class="config-item">
            <p class="config-description">
              After entering your username and password, you can let PluralSync login into your
              account.
            </p>
            <button @click.prevent="loginToVRChat">Login to VRChat</button>
          </div>
          <div class="config-item">
            <label for="vrchat_2fa_code">VRChat 2FA Code ⚠️</label>
            <p class="config-description">
              You may be asked for a Two-Factor-Authentication code. If so, enter it here and submit
              for PluralSync to complete the login.
            </p>
            <input id="vrchat_2fa_code" type="text" v-model="vrchatTwoFactor" />
            <button @click.prevent="submitVRChat2FA">Submit 2FA</button>
          </div>
          <p id="vrchat-login-status">{{ vrchatLoginStatus }}</p>
          <div class="config-item">
            <label for="vrchat_cookie">VRChat Cookie ⚠️</label>
            <p class="config-description">
              This is the VRChat cookie which PluralSync retrieved from VRChat and which it uses to
              update your status. You will not usually need to edit this yourself. It is
              automatically set by PluralSync.
            </p>
            <input
              id="vrchat_cookie"
              type="password"
              :value="config.vrchat_cookie?.secret"
              @input="setSecret('vrchat_cookie', $event)"
            />
          </div>
        </div>
      </div>
      <button type="submit">Save and Restart</button>
      <p id="config-update-status-2">{{ status }}</p>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, type Ref, watch } from 'vue'
import {
  type Decrypted,
  type UserConfigDbEntries,
  type VRChatCredentials,
  type VRChatCredentialsWithTwoFactorAuth,
  type TwoFactorAuthMethod,
  PLURALSYNC_GITHUB_REPOSITORY_RELEASES_URL,
} from '@/pluralsync.bindings'
import { detailed_error_string, http, pluralsync_api } from '@/pluralsync_api'
import { get_privacy_buckets, type PrivacyBucket } from '@/simply_plural_api'

const baseUrl = http.defaults.baseURL!
const config: Ref<UserConfigDbEntries> = ref({} as UserConfigDbEntries)
const defaults: Ref<UserConfigDbEntries> = ref({} as UserConfigDbEntries)
type SecretKeys =
  | 'simply_plural_token'
  | 'vrchat_password'
  | 'vrchat_cookie'
  | 'vrchat_username'
  | 'discord_status_message_token'
  | 'pluralkit_token'

const simply_plural_privacy_buckets: Ref<PrivacyBucket[]> = ref([])
const privacyBucketsStatus = ref('')
const status = ref('')
const vrchatTwoFactor = ref('')
const vrchatLoginStatus = ref('')
const vrchatTmpCookie = ref('')
const vrchatTwoFactorMethod: Ref<TwoFactorAuthMethod | undefined> = ref(undefined)

const VRCHAT_LOGIN_SUCCESSFUL =
  'VRChat login successful and retrieved cookie! Please save config now.'

function setSecret(key: SecretKeys, event: Event) {
  const target = event.target as HTMLInputElement
  if (target.value !== '') {
    config.value[key] = <Decrypted>{ secret: target.value }
  } else {
    config.value[key] = undefined
  }
}

async function loginToVRChat() {
  vrchatLoginStatus.value = 'Requesting 2FA...'
  try {
    const creds: VRChatCredentials = {
      username: config.value.vrchat_username!.secret,
      password: config.value.vrchat_password!.secret,
    }
    const result = await pluralsync_api.vrchat_request_2fa(creds)
    if ('Left' in result) {
      config.value.vrchat_cookie = { secret: result.Left.cookie }
      vrchatLoginStatus.value = VRCHAT_LOGIN_SUCCESSFUL
    } else {
      vrchatTmpCookie.value = result.Right.tmp_cookie
      vrchatTwoFactorMethod.value = result.Right.method
      vrchatLoginStatus.value = `Please enter 2FA code from ${result.Right.method}.`
    }
  } catch (err) {
    console.warn(err)
    vrchatLoginStatus.value = 'Failed to login to VRChat. Error: ' + +detailed_error_string(err)
  }
}

async function submitVRChat2FA() {
  vrchatLoginStatus.value = 'Submitting 2FA code...'
  try {
    const creds_with_tfa: VRChatCredentialsWithTwoFactorAuth = {
      creds: {
        username: config.value.vrchat_username!.secret,
        password: config.value.vrchat_password!.secret,
      },
      code: { inner: vrchatTwoFactor.value },
      tmp_cookie: vrchatTmpCookie.value,
      method: vrchatTwoFactorMethod.value!,
    }
    const result = await pluralsync_api.vrchat_resolve_2fa(creds_with_tfa)
    config.value.vrchat_cookie = { secret: result.cookie }
    vrchatLoginStatus.value = VRCHAT_LOGIN_SUCCESSFUL
  } catch (err) {
    console.warn(err)
    vrchatLoginStatus.value = 'Failed to submit 2FA code. Error: ' + detailed_error_string(err)
  }
}

function copyText(text: string, event: MouseEvent) {
  navigator.clipboard
    .writeText(text)
    .then(() => {
      console.log(`Copied to clipboard: ${text}`)
      const element = event.target as HTMLElement
      element.title = 'Copied!'
    })
    .catch((err) => {
      console.error('Failed to copy text: ', err)
    })
}

async function fetchConfig() {
  try {
    config.value = await pluralsync_api.get_config()
    console.log('Received user config: ', config.value)
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

async function refreshPrivacyBuckets() {
  const token = config.value.simply_plural_token?.secret
  if (!token) {
    return
  }

  privacyBucketsStatus.value = 'Retrieving privacy buckets from Simply Plural ...'
  try {
    simply_plural_privacy_buckets.value = await get_privacy_buckets(token)
    console.log('Privacy buckets:', simply_plural_privacy_buckets.value)
    privacyBucketsStatus.value = 'Your privacy buckets from Simply Plural:'
  } catch (err) {
    console.warn(err)
    simply_plural_privacy_buckets.value = []
    privacyBucketsStatus.value =
      "Couldn't fetch privacy buckets from Simply Plural. Did you correctly set the token? Error: " +
      detailed_error_string(err)
  }
  if (
    config.value.privacy_fine_grained === 'ViaPrivacyBuckets' &&
    (!config.value.privacy_fine_grained_buckets ||
      config.value.privacy_fine_grained_buckets.length === 0)
  ) {
    privacyBucketsStatus.value =
      'Your privacy buckets from Simply Plural are below. Warning: No privacy buckets selected! Nothing will be shown.'
  }
}

onMounted(async () => {
  await fetchConfig()
  await fetchDefaults()
  config.value.simply_plural_token?.secret && (await refreshPrivacyBuckets())
})

watch(
  [() => config.value.simply_plural_token, () => config.value.privacy_fine_grained],
  async () => {
    await refreshPrivacyBuckets()
  },
)
</script>

<style scoped>
.config-container {
  padding: 2rem;
  font-family: sans-serif;
}

.config-section {
  margin-top: 2.5rem;
  border-top: 1px solid #ccc;
  padding-top: 0.5rem;
}

.config-section:last-of-type {
  margin-bottom: 1rem;
  border-bottom: 1px solid #ccc;
  padding-bottom: 0.5rem;
}

.config-grid {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-top: 1rem;
}

.config-item {
  display: flex;
  flex-direction: column;
}

.config-item label {
  font-weight: bold;
  margin-bottom: 0.5rem;
}

.config-item input,
.config-item select {
  margin-top: 0.2rem;
  margin-bottom: 0.2rem;
  padding: 0.5rem;
  border: 1px solid #ccc;
  border-radius: 4px;
}
.config-item button {
  width: 10rem;
  font-size: smaller;
  background-color: gray;
}

.config-item button:hover {
  background-color: black;
}

.copyable {
  cursor: pointer;
  text-decoration: underline dotted;
}

.config-description {
  font-size: smaller;
}

.warning {
  font-weight: bold;
  color: #810000ff;
}

button {
  margin-top: 1.5rem;
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 4px;
  background-color: #007bff;
  color: white;
  font-weight: bold;
  cursor: pointer;
}

button:hover {
  background-color: #0056b3;
}

.config-item img {
  max-width: 100%;
  border: 1px solid #ccc;
  margin-top: 0.5rem;
}

/* better visual checkboxes */
.config-item input[type='checkbox'] {
  /* Reset default appearance */
  -webkit-appearance: none;
  -moz-appearance: none;
  appearance: none;

  /* Dimensions and positioning */
  position: relative;
  align-self: flex-start;
  width: 3.5rem;
  height: 1.75rem;

  /* Styling the track */
  background-color: #ccc;
  border-radius: 1.75rem;
  cursor: pointer;
  transition: background-color 0.2s ease-in-out;
}

.config-item input[type='checkbox']::before {
  content: '';
  position: absolute;
  top: 0.15rem;
  left: 0.15rem;
  width: 1.45rem;
  height: 1.45rem;
  background-color: white;
  border-radius: 50%;
  transition: left 0.2s ease-in-out;
}

.config-item input[type='checkbox']:checked {
  background-color: #007bff;
}

.config-item input[type='checkbox']:checked::before {
  /* Move the toggle to the right */
  left: 1.9rem;
}
</style>
