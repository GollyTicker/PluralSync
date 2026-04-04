import { invoke } from '@tauri-apps/api/core'
import { type PluralSyncVariantInfo } from './pluralsync.bindings'

export async function fetchAndRenderVersions(): Promise<[string, PluralSyncVariantInfo]> {
  let bridgeVersion = await getBridgeVersion()

  let bridge_version_element = document.querySelector<HTMLParagraphElement>('#client-version')!
  bridge_version_element.innerText = 'client: ' + bridgeVersion

  console.log('fetch_base_url_and_variant_info ...')
  let result = await invoke<[string, PluralSyncVariantInfo]>('fetch_base_url_and_variant_info')
  console.log('fetch_base_url_and_variant_info.0:', result[0])
  console.log('fetch_base_url_and_variant_info.1:', result[1])

  let element = document.querySelector<HTMLParagraphElement>('#variant-info')!

  let { show_in_ui, variant, description, version } = result[1]

  element.style.display = show_in_ui ? 'inline-block' : 'none'
  element.innerText = 'server: @' + variant + ' ' + version
  if (description) {
    element.title = description
  }

  if (bridgeVersion !== result[1].version) {
    document.querySelector<HTMLDivElement>('#update-bridge-note')!.innerText =
      '⚠️ PluralSync-Bridge is outdated. Install the newest version from PluralSync Website! ⚠️'
  }

  return result
}

export async function getBridgeVersion(): Promise<string> {
  return await invoke<string>('get_bridge_version')
}
