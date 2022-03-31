import React from 'react'
import { useInvoke } from './hooks/swr'
import './App.scss'

const App = () => {
  const { data: playing, update: togglePlay } = useInvoke(
    {},
    'get_paused',
    'toggle_play'
  )

  return (
    <div className="App">
      <button onClick={togglePlay}>
        {playing ? <>Play</> : <>Pause</> }
      </button> 
    </div>
  )
}

export default App
