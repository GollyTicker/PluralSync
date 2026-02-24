import { invoke } from '@tauri-apps/api/core'
import router from '../router'
import { CANONICAL_PLURALSYNC_BASE_URL, type UserLoginCredentials } from '../pluralsync.bindings'
import { fetchAndRenderVariantInfo } from '../variant-info'

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
  const pluralsyncBaseUrlInput = document.querySelector<HTMLInputElement>(
    '#pluralsync-base-url-input',
  )!

  let [baseUrl, _] = await fetchAndRenderVariantInfo()
  pluralsyncBaseUrlInput.value = baseUrl

  loginForm?.addEventListener('submit', async (e) => {
    e.preventDefault()

    const email = document.querySelector<HTMLInputElement>('#email')!.value
    const password = document.querySelector<HTMLInputElement>('#password')!.value
    const baseUrl = pluralsyncBaseUrlInput.value

    loginStatus.textContent = 'Logging in ...'

    if (email && password) {
      try {
        const version = await invoke<string>('get_bridge_version')
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
}
