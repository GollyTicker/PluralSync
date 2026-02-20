<template>
  <div class="config-section">
    <h2>Website</h2>
    <div class="config-grid">
      <div class="config-item">
        <label for="enable_website">Enable Website</label>
        <p class="config-description">
          Let PluralSync display your fronting status (avatars included) as a webpage. Others can
          simply open the link and see the current fronters without needing to be logged in in any
          of the other platforms.
          <br />
          Note: Not all images are created equally. Some platforms hosting images don't allow you to
          embed the image onto all websites, but only to specific websites or their own. Most
          notably, if you uploaded an image to SimplyPlural and use that, then PluralSync can't use
          that. You would need to upload the avatar image or a classic hosting website such as imgur
          or imgbb and then set that image URL as the avatar URL in SimplyPlural. PluralSync will
          then use that URL as well and it should work.
        </p>
        <input id="enable_website" type="checkbox" v-model="config.enable_website" />
      </div>
      <div class="config-item">
        <label for="website_system_name">System Name</label>
        <p class="config-description">The name of your system as the title of the website view.</p>
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
          Adapt the link at which your fronting website is shown. For example, if you want your link
          to be "{{ baseUrl }}/fronting/ocean-collective", then set this field to
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
</template>

<!-- eslint-disable vue/no-mutating-props -->
<script setup lang="ts">
import type { UserConfigDbEntries } from '@/pluralsync.bindings'
import { http } from '@/pluralsync_api'

interface Props {
  config: UserConfigDbEntries
  defaults: UserConfigDbEntries
}

defineProps<Props>()

const baseUrl = http.defaults.baseURL!
</script>
