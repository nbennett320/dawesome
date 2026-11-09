import React from 'react'
import { useAppSelector, useAppDispatch } from './hooks/redux'
import Topbar from './components/topbar/Topbar'
import Sidebar from './components/sidebar/Sidebar'
import TabWindow from './views/TabWindow'
import { listeners } from './events/window'
import { selectDevicePreferences } from './state/slices/windowSlice'
import { projectName, selectProjectPath } from './state/slices/projectSlice'
import ProjectNotice from './components/ProjectNotice'

const App = () => {
  const devicePreferences = useAppSelector(selectDevicePreferences)
  const dispatch = useAppDispatch()
  const projectPath = useAppSelector(selectProjectPath)

  React.useEffect(() => {
    listeners.devicePreferences(devicePreferences, dispatch)
  }, [devicePreferences])

  // File menu: new, open, open recent, save, save as. Window menu: open tabs
  React.useEffect(() => {
    const listening = [
      listeners.projectMenu(dispatch),
      listeners.recentProjectMenu(dispatch),
      listeners.viewMenu(dispatch),
    ]
    return () => { listening.forEach(l => l.then(stop => stop())) }
  }, [dispatch])

  // show the project's name in the window's title bar
  React.useEffect(() => {
    const title = `${projectName(projectPath) ?? 'Untitled'} — dawesome`
    // loaded when needed, since it expects to run inside the app's window
    import('@tauri-apps/api/window')
      .then(({ appWindow }) => appWindow.setTitle(title))
      // eslint-disable-next-line no-console
      .catch(e => console.error('could not set the window title:', e))
  }, [projectPath])

  return (
    <div className="AppRoot min-h-screen text-gray-800">
      <Topbar />
      <ProjectNotice />
      <div className="WorkspaceContainer flex flex-row w-full">
        <Sidebar />
        <div className="w-full h-full min-w-0">
          <TabWindow />
        </div>
      </div>
    </div>
  )
}

export default App
