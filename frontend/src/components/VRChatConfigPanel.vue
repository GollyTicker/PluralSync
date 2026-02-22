<template>
  <div class="config-section">
    <h2>VRChat</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="enable_vrchat">Enable VRChat Status Message ⚠️</label>
        <input id="enable_vrchat" type="checkbox" v-model="config.enable_vrchat" />
        <p class="config-description">
          Shows the fronting status on VRChat in the custom status at your profile in VR and on the
          website.
          <br />
          For that, you will need to login into VRChat such that PluralSync can set the fronting
          status on VRChat's side.
          <br />
          <span class="warning"
            >WARNING! This violates VRChat Terms of Service. Use at your own risk! This option might
            be removed at any point!</span
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
          After entering your username and password, you can let PluralSync login into your account.
        </p>
        <button @click.prevent="loginToVRChat">Login to VRChat</button>
      </div>
      <div class="config-item">
        <label for="vrchat_2fa_code">VRChat 2FA Code ⚠️</label>
        <p class="config-description">
          You may be asked for a Two-Factor-Authentication code. If so, enter it here and submit for
          PluralSync to complete the login.
        </p>
        <input id="vrchat_2fa_code" type="text" v-model="vrchatTwoFactor" />
        <button @click.prevent="submitVRChat2FA">Submit 2FA</button>
      </div>
      <p id="vrchat-login-status">{{ vrchatLoginStatus }}</p>
      <div class="config-item">
        <label for="vrchat_cookie">VRChat Cookie ⚠️</label>
        <p class="config-description">
          This is the VRChat cookie which PluralSync retrieved from VRChat and which it uses to
          update your status. You will not usually need to edit this yourself. It is automatically
          set by PluralSync.
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
</template>

<script setup lang="ts">
import { ref, type Ref } from 'vue'
import type {
  UserConfigDbEntries,
  VRChatCredentials,
  VRChatCredentialsWithTwoFactorAuth,
  TwoFactorAuthMethod,
  Decrypted,
} from '@/pluralsync.bindings'
import { detailed_error_string, pluralsync_api } from '@/pluralsync_api'

interface Props {
  config: UserConfigDbEntries
}

const props = defineProps<Props>()

const vrchatTwoFactor = ref('')
const vrchatLoginStatus = ref('')
const vrchatTmpCookie = ref('')
const vrchatTwoFactorMethod: Ref<TwoFactorAuthMethod | undefined> = ref(undefined)

const VRCHAT_LOGIN_SUCCESSFUL =
  'VRChat login successful and retrieved cookie! Please save config now.'

type SecretKeys = 'vrchat_username' | 'vrchat_password' | 'vrchat_cookie'

function setSecret(key: SecretKeys, event: Event) {
  const target = event.target as HTMLInputElement
  if (target.value !== '') {
    props.config[key] = <Decrypted>{ secret: target.value }
  } else {
    props.config[key] = undefined
  }
}

async function loginToVRChat() {
  vrchatLoginStatus.value = 'Requesting 2FA...'
  try {
    const creds: VRChatCredentials = {
      username: props.config.vrchat_username!.secret,
      password: props.config.vrchat_password!.secret,
    }
    const result = await pluralsync_api.vrchat_request_2fa(creds)
    if ('Left' in result) {
      props.config.vrchat_cookie = { secret: result.Left.cookie }
      vrchatLoginStatus.value = VRCHAT_LOGIN_SUCCESSFUL
    } else {
      vrchatTmpCookie.value = result.Right.tmp_cookie
      vrchatTwoFactorMethod.value = result.Right.method
      vrchatLoginStatus.value = `Please enter 2FA code from ${result.Right.method}.`
    }
  } catch (err) {
    console.warn(err)
    vrchatLoginStatus.value = 'Failed to login to VRChat. Error: ' + detailed_error_string(err)
  }
}

async function submitVRChat2FA() {
  vrchatLoginStatus.value = 'Submitting 2FA code...'
  try {
    const creds_with_tfa: VRChatCredentialsWithTwoFactorAuth = {
      creds: {
        username: props.config.vrchat_username!.secret,
        password: props.config.vrchat_password!.secret,
      },
      code: { inner: vrchatTwoFactor.value },
      tmp_cookie: vrchatTmpCookie.value,
      method: vrchatTwoFactorMethod.value!,
    }
    const result = await pluralsync_api.vrchat_resolve_2fa(creds_with_tfa)
    props.config.vrchat_cookie = { secret: result.cookie }
    vrchatLoginStatus.value = VRCHAT_LOGIN_SUCCESSFUL
  } catch (err) {
    console.warn(err)
    vrchatLoginStatus.value = 'Failed to submit 2FA code. Error: ' + detailed_error_string(err)
  }
}
</script>

<style scoped>
@import url('../assets/config.css');
</style>
