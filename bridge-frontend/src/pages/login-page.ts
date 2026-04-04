import { invoke } from '@tauri-apps/api/core'
import router from '../router'
import { CANONICAL_PLURALSYNC_BASE_URL, type UserLoginCredentials } from '../pluralsync.bindings'
import { fetchAndRenderVersions, getBridgeVersion } from '../variant-info'
import { check } from '@tauri-apps/plugin-updater'

export async function renderLoginPage() {
  document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
    <div>
      <h1>Login</h1>
      <div id="login-status">Not logged in</div>
      <form id="login-form">
        <input type="email" id="email" placeholder="Email" required />
        <input type="password" id="password" placeholder="Password" required />
        <div id="variant-container">
          <span>Variant</span>
          <input type="text" id="pluralsync-base-url-input" placeholder="${CANONICAL_PLURALSYNC_BASE_URL}" />
        </div>
        <button type="submit">Login</button>
      </form>
    </div>
  `

  const loginForm = document.querySelector<HTMLFormElement>('#login-form')!
  const loginStatus = document.querySelector<HTMLDivElement>('#login-status')!
  const updaterStatus = document.querySelector('#updater-status')!
  const pluralsyncBaseUrlInput = document.querySelector<HTMLInputElement>(
    '#pluralsync-base-url-input',
  )!

  let [baseUrl, _] = await fetchAndRenderVersions()
  pluralsyncBaseUrlInput.value = baseUrl

  loginForm?.addEventListener('submit', async (e) => {
    e.preventDefault()

    const email = document.querySelector<HTMLInputElement>('#email')!.value
    const password = document.querySelector<HTMLInputElement>('#password')!.value
    const baseUrl = pluralsyncBaseUrlInput.value

    loginStatus.textContent = 'Logging in ...'

    if (email && password) {
      try {
        const version = await getBridgeVersion()
        let creds: UserLoginCredentials = {
          email: { inner: email },
          password: { inner: { inner: password } },
          client_version: version,
        }
        await invoke('store_credentials', { creds, baseUrl })
        await invoke('login_with_stored_credentials')
        router.navigate('/') // let the start page login again
      } catch (error: any) {
        console.warn(error)
        let original_error_text: string = error.toString()
        let user_friendly = original_error_text.includes('403 Forbidden')
          ? 'Invalid login. Please try again.'
          : `Login failed: ${original_error_text}`
        loginStatus!.textContent = user_friendly
      }
    }
  })

  check()
    .then(async (update) => {
      if (update !== null) {
        updaterStatus.textContent = '⚠️ Update ${update.version} available. Installing...'
        await update.downloadAndInstall()
        updaterStatus.textContent = '⚠️ Bridge Updated. Please restart!'
        loginStatus.textContent = '⚠️ Bridge Updated. Please restart!'
        // todo. adapt notice to manual intervention on untested platforms
      } else {
        updaterStatus.textContent = '✅ Up to date'
      }
      return Promise.resolve()
    })
    .catch((e) => {
      updaterStatus.textContent = '❌ Update check failed: ${e}'
      console.error(e)
    })
}
