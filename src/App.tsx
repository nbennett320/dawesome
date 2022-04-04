import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useInvoke } from './hooks/swr'
import './App.scss'

const App = () => {
  // const [tempo, setTempo] = React.useState(() => invoke('get_playlist_tempo').then(x => x))
  const [tempo, setTempo] = React.useState(120.)
  const [runtime, setRuntime] = React.useState<number>()
  const timer = React.useRef<any>(null)
  const { data: playing, update: togglePlay } = useInvoke(
    {},
    'get_playlist_playing',
    'toggle_playlist'
  )

  // const { data: tempo, update: setTempo } = useInvoke(
  //   { val: 120 },
  //   'get_playlist_tempo',
  //   'set_playlist_tempo'
  // )
  
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

  React.useEffect(() => {
    invoke('set_playlist_tempo', { val: tempo as any })
  }, [tempo])

  return (
    <div className="App">
      <button onClick={togglePlay}>
        {playing ? <>Pause</> : <>Play</> }
      </button> 

      <label>Tempo:</label>
      <input type='number' value={tempo as any} onInput={(e) => { setTempo(parseFloat(e.currentTarget.value) as number) }}>
      </input>

      {playing && <div style={{ display: 'flex', flexDirection: 'column', width: '200px', marginLeft: 'auto', marginRight: 'auto'}}>
        <span>Playlist runtime:</span> <span style={{ display: 'flex',flexDirection: 'row' }}><span style={{ width: '100px',left: '0' }}>{runtime}</span> <span style={{ left: '100%' }}>ms</span></span>
      </div>}
    </div>
  )
}

export default App
