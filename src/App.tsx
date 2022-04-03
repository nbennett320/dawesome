import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useInvoke } from './hooks/swr'
import './App.scss'

const App = () => {
  const [runtime, setRuntime] = React.useState<number>()
  const timer = React.useRef<any>(null)
  const { data: playing, update: togglePlay } = useInvoke(
    {},
    'get_playlist_playing',
    'toggle_playlist'
  )
  
  React.useEffect(() => {
    if(playing) {
      timer.current = setInterval(() => {
        invoke('get_playlist_start_time').then((data) => {
          const res = (Date.now().valueOf()) - (data as number * 1000)
          setRuntime(res)
        })
      }, 100)
    } else {
      clearInterval(timer.current)
    }
  }, [playing])

  return (
    <div className="App">
      <button onClick={togglePlay}>
        {playing ? <>Pause</> : <>Play</> }
      </button> 
      {playing && <div style={{ display: 'flex', flexDirection: 'column' }}>
        Playlist runtime: <span>{runtime}</span>
      </div>}
    </div>
  )
}

export default App
