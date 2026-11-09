import { listen } from '@tauri-apps/api/event'
import { setShowDevicePreferences } from '../state/slices/windowSlice'
import { ProjectCommand, openRecentProject, runProjectCommand } from '../state/slices/projectSlice'
import type { AppDispatch } from '../state/store'

const devicePreferences = async (selector: any, dispatcher: any) => {
  await listen('menu-device-preference', ev => {
    dispatcher(setShowDevicePreferences(!selector))
  })
}
// the File menu's project items. resolves to a function that stops listening
const projectMenu = (dispatch: AppDispatch) =>
  listen<{ data: ProjectCommand }>('menu-project', ev => {
    dispatch(runProjectCommand(ev.payload.data))
  })

// File › Open Recent's projects. resolves to a function that stops listening
const recentProjectMenu = (dispatch: AppDispatch) =>
  listen<{ data: string }>('menu-open-recent', ev => {
    dispatch(openRecentProject(ev.payload.data))
  })

const listeners = { 
  devicePreferences,
  projectMenu,
  recentProjectMenu,
}

export { listeners }

