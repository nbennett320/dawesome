import { listen } from '@tauri-apps/api/event'
import { openTab, setShowDevicePreferences } from '../state/slices/windowSlice'
import { ProjectCommand, openRecentProject, runProjectCommand } from '../state/slices/projectSlice'
import type { AppDispatch } from '../state/store'
import { View } from '../types/ui'

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

// the Window menu's items, by the names the engine sends
const menuViews: Record<string, View> = {
  playlist: View.Playlist,
  sample: View.SamplePreview,
  mixer: View.Mixer,
}

// the Window menu: open a tab, or select it where it's already open.
// resolves to a function that stops listening
const viewMenu = (dispatch: AppDispatch) =>
  listen<{ data: string }>('menu-open-view', ev => {
    const view = menuViews[ev.payload.data]
    if(view !== undefined) dispatch(openTab(view))
  })

const listeners = { 
  devicePreferences,
  projectMenu,
  recentProjectMenu,
  viewMenu,
}

export { listeners }

