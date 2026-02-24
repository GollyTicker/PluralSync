import { invoke } from '@tauri-apps/api/core'
import router from '../router'
import type { JwtString } from '../pluralsync.bindings'
import { listen } from '@tauri-apps/api/event'
import * as tauriAutoStartPlugin from '@tauri-apps/plugin-autostart'
import { check } from '@tauri-apps/plugin-updater'

const WEBSOCKET_RETRY_INTERVAL_MILLIS = 10 * 1000

const AUTOSTART_IS_ENABLED_TEXT =
  'PluralSync-Bridge will automatically start when you start the computer.'
const AUTOSTART_IS_DISABLED_TEXT =
  'It is recommended to set PluralSync-Bridge to automatically start when you start the computer.'

let retryTimer: NodeJS.Timeout | undefined

export async function renderStatusPage() {
  const version = await invoke<string>('get_bridge_version')

  document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
    <div>
      <h1>PluralSync Bridge</h1>
      <div>Version: ${version}</div>
      <button id="check-updates">Check for Updates</button>
      <div id="updater-status"></div>
      <div id="bridge-status">Connecting to PluralSync Server and local Discord client ...</div>
      <div>
        <div class="autostart-container">
          <label for="autostart">Checking autostart....</label>
          <input type="checkbox" id="autostart" />
        </div>
      </div>
      <button id="logout-button">Disconnect and Logout</button>
    </div>
  `

  const logoutButton = document.querySelector<HTMLButtonElement>('#logout-button')
  logoutButton?.addEventListener('click', async () => {
    localStorage.removeItem('jwt')
    try {
      await invoke('stop_and_clear_credentials')
    } catch (e) {
      console.warn('Failed to stop and clear credentials', e)
    }
    router.navigate('/')
  })

  document.querySelector('#check-updates')?.addEventListener('click', async () => {
    try {
      const update = await check()
      if (update) {
        if (confirm(`Update ${update.version} available. Install now?`)) {
          await update.downloadAndInstall()
        }
      } else {
        document.querySelector('#updater-status')!.textContent = 'Up to date'
      }
    } catch (e) {
      console.error(e)
      document.querySelector('#updater-status')!.textContent = 'Update check failed'
    }
  })

  await refreshAutostartSection()

  router.addLeaveHook('/status', (done: Function) => {
    clearTimeout(retryTimer)
    done()
  })

  /* no await */ subscribe_to_bridge_channel()
}

async function subscribe_to_bridge_channel() {
  try {
    const jwt: JwtString = JSON.parse(localStorage.getItem('jwt')!)
    await invoke('subscribe_to_bridge_channel', { jwt })
    bridgeStatus().textContent = 'Connected to PluralSync and receiving updates...'
  } catch (e) {
    console.warn(e)
    restart_websocket_connection_after_retry_interval()
    bridgeStatus().textContent = `Failed to connect to PluralSync: ${e}. Retrying in ${WEBSOCKET_RETRY_INTERVAL_MILLIS / 1000} seconds...`
  }
}

async function refreshAutostartSection() {
  let isEnabled = await tauriAutoStartPlugin.isEnabled()
  autoStartLabel().innerText = isEnabled ? AUTOSTART_IS_ENABLED_TEXT : AUTOSTART_IS_DISABLED_TEXT
  autoStartCheckbox().checked = isEnabled

  autoStartCheckbox().addEventListener('change', async () => {
    if (await tauriAutoStartPlugin.isEnabled()) {
      await tauriAutoStartPlugin.disable()
    } else {
      await tauriAutoStartPlugin.enable()
    }
    refreshAutostartSection()
  })
}

function restart_websocket_connection_after_retry_interval() {
  retryTimer = setTimeout(subscribe_to_bridge_channel, WEBSOCKET_RETRY_INTERVAL_MILLIS)
}

function bridgeStatus(): HTMLDivElement {
  return document.querySelector<HTMLDivElement>('#bridge-status')!
}

function autoStartLabel(): HTMLLabelElement {
  return document.querySelector<HTMLLabelElement>('.autostart-container label')!
}

function autoStartCheckbox(): HTMLInputElement {
  return document.querySelector<HTMLInputElement>('#autostart')!
}

listen<string>('notify_user_on_status', (event) => {
  bridgeStatus().textContent = event.payload
})

listen<number>(
  'restart_websocket_connection_after_retry_interval',
  restart_websocket_connection_after_retry_interval,
)
