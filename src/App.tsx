import React from 'react'
import { useAppSelector, useAppDispatch } from './hooks/redux'
import Topbar from './components/topbar/Topbar'
import Sidebar from './components/sidebar/Sidebar'
import TabWindow from './views/TabWindow'
import { listeners } from './events/window'
import { selectDevicePreferences } from './state/slices/windowSlice'

const App = () => {
  const devicePreferences = useAppSelector(selectDevicePreferences)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    listeners.devicePreferences(devicePreferences, dispatch)
  }, [devicePreferences])

  return (
    <div className="AppRoot min-h-screen text-gray-800">
      <Topbar />
      <div className="WorkspaceContainer flex flex-row w-full">
        <Sidebar />
        <div className="w-full">
          <TabWindow />
        </div>
      </div>
    </div>
  )
}

export default App
