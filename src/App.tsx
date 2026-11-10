import React from 'react'
import { useAppSelector, useAppDispatch } from './hooks/redux'
import Topbar from './components/topbar/Topbar'
import Sidebar from './components/sidebar/Sidebar'
import TabWindow from './views/TabWindow'
import { listeners } from './events/window'
import { projectName, selectProjectPath } from './state/slices/projectSlice'
import ProjectNotice from './components/ProjectNotice'
import DragGhost from './components/DragGhost'

const App = () => {
  const dispatch = useAppDispatch()
  const projectPath = useAppSelector(selectProjectPath)

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
    const title = `${projectName(projectPath) ?? 'Untitled'} — Dawesome`
    // loaded when needed, since it expects to run inside the app's window
    import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => getCurrentWindow().setTitle(title))
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
      <DragGhost />
    </div>
  )
}

export default App
