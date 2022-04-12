import React from 'react'
import Topbar from './components/topbar/Topbar'
import Playlist from './views/playlist/Playlist'

const App = () => (
  <div className="min-h-screen text-gray-800">
    <Topbar />
    <Playlist />
  </div>
)

export default App
