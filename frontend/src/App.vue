<template>
  <div id="app-container">
    <nav>
      <div id="app-header">
        <img src="/favicon.png" alt="logo" />
        <h1 class="title">PluralSync</h1>
        <p
          v-if="variantInfo?.show_in_ui"
          id="variant-info"
          :title="variantInfo.description ?? undefined"
        >
          @{{ variantInfo.variant }}
        </p>
      </div>
      <div class="nav-links-container">
        <router-link v-if="loggedIn" to="/status">Status</router-link>
        <router-link v-if="loggedIn" to="/history">History</router-link>
        <router-link v-if="loggedIn" to="/config">Settings</router-link>
        <router-link v-if="loggedIn" to="/logout">Logout</router-link>
      </div>
    </nav>
    <router-view class="content" />
    <footer>
      <LicenseInfo />
    </footer>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import LicenseInfo from './components/LicenseInfo.vue'
import { pluralsync_api } from './pluralsync_api'
import type { PluralSyncVariantInfo } from './pluralsync.bindings'
import { loggedIn } from './jwt'

const variantInfo = ref<PluralSyncVariantInfo | null>(null)

onMounted(async () => {
  variantInfo.value = await pluralsync_api.get_variant_info()
})
</script>

<style scoped>
#app-container {
  padding-top: 60px;
  /* Add padding to prevent content from overlapping with the nav bar */
  display: flex;
  flex-direction: column;
  width: 100%;
  min-height: 100vh;
  box-sizing: border-box;
}

nav {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 60px;
  padding: 0 1em;
  background-color: #f8f9fa;
  border-bottom: 2px solid #dee2e6;
  z-index: 1000;
  display: flex;
  justify-content: flex-start;
  align-items: center;
}

.nav-links-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 0.5rem;
}

.nav-links-container a {
  font-weight: bold;
  text-decoration: none;
  padding: 0.5em 1em;
  border: 1px solid transparent;
  border-radius: 0.25rem;
  color: black;
  background-color: var(--color-background-mute);
}

#variant-info {
  padding: 0.25rem 0.75rem;
  border-radius: 0.2em;
  background-color: #8962d1;
  color: white;
  text-align: center;
  cursor: help;
  margin-left: 0.5rem;
  font-size: 0.9em;
  font-weight: 600;
  display: inline-block;
  white-space: nowrap;
}

nav a:hover {
  color: var(--color-primary);
}

#app-header {
  display: flex;
  align-items: center;
  margin-right: 2rem;
}

#app-header img {
  width: 40px;
  height: 40px;
  margin-right: 1rem;
}

#app-header .title {
  font-size: 2em;
  font-weight: 700;
  margin: 0;
}

.content {
  max-width: 1280px;
  margin: 0 auto;
}

footer {
  margin-top: auto;
  padding: 1rem;
  text-align: center;
  color: darkslategray;
  background-color: azure;
}

/* for the slim mobile screens */
@media (max-width: 768px) {
  nav {
    flex-direction: column;
    height: auto;
    align-items: flex-start;
  }

  #app-container {
    /* 60px for header + ~60px for nav links */
    padding-top: 120px;
  }

  #app-header {
    margin-right: 0;
    width: 100%;
  }

  .nav-links-container {
    width: 100%;
    flex-wrap: wrap;
    padding-bottom: 0.5rem;
  }
}
</style>
