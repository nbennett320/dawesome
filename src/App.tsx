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
      }, 1000)
    } else {
      clearInterval(timer.current)
    }
  }, [playing])

  return (
    <div className="App">
      <button onClick={togglePlay}>
        {playing ? <>Pause</> : <>Play</> }
      </button> 
      {playing && <div style={{ display: 'flex', flexDirection: 'column', width: '200px', marginLeft: 'auto', marginRight: 'auto'}}>
        <span>Playlist runtime:</span> <span style={{ display: 'flex',flexDirection: 'row' }}><span style={{ width: '100px',left: '0' }}>{runtime}</span> <span style={{ left: '100%' }}>ms</span></span>
      </div>}
    </div>
  )
}

export default App
