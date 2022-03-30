import React from 'react'
import { useInvoke } from './hooks/swr'
import './App.scss'

const App = () => {
  const counter_args = { delta: 1 }
  const play_args = { delta: 1 }

  const { data: counter, update: increment } = useInvoke(
    counter_args,
    'get_counter',
    'increment_counter'
  )

  const { data: is_playing, update: toggle_play } = useInvoke(
    play_args,
    'get_counter',
    'toggle_play_sound'
  )
  
  return (
    <div className="App">
      times played: {counter}
      <br />
      <button onClick={increment}>
        Try counter
      </button> 
      <br />
      is playing: {is_playing}
      <br />
      <button onClick={toggle_play}>
        Play
      </button> 
    </div>
  )
}

export default App
