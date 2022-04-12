import { listen } from '@tauri-apps/api/event'
import { setShowDevicePreferences } from '../state/slices/windowSlice'

const devicePreferences = async (selector: any, dispatcher: any) => {
  await listen('menu-device-preference', ev => {
    dispatcher(setShowDevicePreferences(!selector))
  })
}
const listeners = { 
  devicePreferences
}

export { listeners }

