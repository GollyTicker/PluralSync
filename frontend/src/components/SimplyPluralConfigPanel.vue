<template>
  <div class="config-section">
    <h2>Simply Plural</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="simply_plural_token">Simply Plural Token</label>
        <p class="config-description">
          The private READ-token used by PluralSync to access your Simply Plural system to check for
          changes. To make one, open
          <a href="https://app.apparyllis.com/" target="_blank">SimplyPlural</a>, go to Settings >
          Account > Tokens, create a READ token and copy-paste it here.
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
          The following toggles and settings allow you to configure the privacy and visibility of
          the members and custom fronts. The "Show ..." toggles are used to show/hide categories of
          fronts. If they're OFF, then nothing of that category is shown.
          <br />
          Each fronter must pass all conditions detailed below to be shown. E.g. if a fronter is a
          non-archived member with the setting "Prevent notifications on front change" enabled in
          SimplyPlural, then member will be shown exactly when (1) the member is fronting AND (2)
          "Show Active Members" is ON AND (3) Respect "Prevent notifications on front change" is
          OFF. If any of the above conditions are not met, then the member is not shown.
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
          Show <span style="font-weight: bold">archived</span> members. They might still be hidden,
          if the other conditions make them hidden.
        </p>
        <input id="show_members_archived" type="checkbox" v-model="config.show_members_archived" />
      </div>
      <div class="config-item">
        <label for="respect_front_notifications_disabled">
          Respect "Prevent notifications on front change"
        </label>
        <p class="config-description">
          If ON, then the member will be hidden, if their fronting change is configured not notify
          others. If OFF, then this setting in Simply Plural is ignored.
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
            PluralSync friend is assigned to. The above "Show" toggles still apply. (Note, that the
            privacy settings you can configure for friends like "They can see your shared members"
            etc are IGNORED. Only the privacy buckets are used.)
          </li>
          <li>
            Directly choose the privacy buckets on this PluralSync Website here and any fronts
            assigned to the privacy buckets selected here will be shown. The above "Show" toggles
            still apply.
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
          If you choose "via privacy buckets" above, then you can configure which privacy buckets to
          use here. You can chose multiple privacy buckets.

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
</template>

<!-- eslint-disable vue/no-mutating-props -->
<script setup lang="ts">
import { ref, watch, type Ref } from 'vue'
import type { UserConfigDbEntries, Decrypted } from '@/pluralsync.bindings'
import { detailed_error_string } from '@/pluralsync_api'
import { get_privacy_buckets, type PrivacyBucket } from '@/simply_plural_api'

interface Props {
  config: UserConfigDbEntries
}

const props = defineProps<Props>()

const simply_plural_privacy_buckets: Ref<PrivacyBucket[]> = ref([])
const privacyBucketsStatus = ref('')

type SecretKeys = 'simply_plural_token'

function setSecret(key: SecretKeys, event: Event) {
  const target = event.target as HTMLInputElement
  if (target.value !== '') {
    props.config[key] = <Decrypted>{ secret: target.value }
  } else {
    props.config[key] = undefined
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

async function refreshPrivacyBuckets() {
  const token = props.config.simply_plural_token?.secret
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
    props.config.privacy_fine_grained === 'ViaPrivacyBuckets' &&
    (!props.config.privacy_fine_grained_buckets ||
      props.config.privacy_fine_grained_buckets.length === 0)
  ) {
    privacyBucketsStatus.value =
      'Your privacy buckets from Simply Plural are below. Warning: No privacy buckets selected! Nothing will be shown.'
  }
}

watch(
  [() => props.config.simply_plural_token, () => props.config.privacy_fine_grained],
  async () => {
    await refreshPrivacyBuckets()
  },
)
</script>
