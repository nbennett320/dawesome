import React from 'react'
import { useInvoke } from './hooks/swr'
import './App.css'

const App = () => {
  const defaultArgs = { delta: 1 }

  const { data: counter, update: togglePlay } = useInvoke(
    defaultArgs,
    'get_counter',
    'play_sound'
  )
  
  return (
    <div className="App">
      times played: {counter}
      <br />
      <button onClick={togglePlay}>
        Play
      </button> 
    </div>
  )
}

export default App
