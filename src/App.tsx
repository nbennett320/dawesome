import React from 'react'
import './App.scss'

import { invoke } from '@tauri-apps/api/tauri'
const App = () => {

  return (
    <div className="App">
      <button onClick={() => invoke('toggle_play')}>
        Play
      </button> 
    </div>
  )
}

export default App
