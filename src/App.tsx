import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useInvoke } from './hooks/swr'

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

  React.useEffect(() => {
    if(playing) togglePlay({})
    invoke('set_playlist_tempo', { val: tempo as any })
  }, [tempo])

  return (
    <div className="grid p-3 place-content-center">
      <button type='button' className='p-3 text-white bg-indigo-700 rounded-sm' onClick={togglePlay}>
        {playing ? <>Pause</> : <>Play</> }
      </button> 

      <span>Tempo:</span>
      <input type='number' value={tempo as any} onInput={(e) => { setTempo(parseFloat(e.currentTarget.value) as number) }} />

      {playing && <div style={{ display: 'flex', flexDirection: 'column', width: '200px', marginLeft: 'auto', marginRight: 'auto'}}>
        <span>Playlist runtime:</span> <span style={{ display: 'flex',flexDirection: 'row' }}><span style={{ width: '100px',left: '0' }}>{runtime}</span> <span style={{ left: '100%' }}>ms</span></span>
      </div>}
    </div>
  )
}

export default App
