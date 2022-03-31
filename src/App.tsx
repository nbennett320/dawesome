import React from 'react'
import { useInvoke } from './hooks/swr'

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
    <div className="grid place-content-center p-3">
      times played: {counter}
      <br />
      <button className="bg-indigo-700 text-white p-3 rounded-sm" onClick={increment}>
        Try counter
      </button>
      <br />
      is playing: {is_playing}
      <br />
      <button className="bg-violet-700 text-white p-3 rounded-sm" onClick={toggle_play}>
        Play
      </button>
    </div>
  )
}

export default App
