/* eslint-disable jsx-a11y/label-has-associated-control */
import React, { useRef, useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api'
import { useInvoke } from './hooks/swr'

const App = () => {
  // const [tempo, setTempo] = React.useState(() => invoke('get_playlist_tempo').then(x => x))
  const [tempo, setTempo] = useState(120)
  const [runtime, setRuntime] = useState<number>()
  const timer: { current: NodeJS.Timeout | null } = useRef(null)
  const { data: playing, update: togglePlay } = useInvoke({}, 'get_playlist_playing', 'toggle_playlist')

  useEffect(() => {
    if (playing) {
      timer.current = setInterval(() => {
        invoke('get_playlist_start_time').then((data) => {
          const res = Date.now().valueOf() - (data as number) * 1000
          setRuntime(res)
        })
      }, 100)
    } else {
      clearInterval(timer.current as NodeJS.Timeout)
    }
  }, [playing])

  useEffect(() => {
    if (playing) togglePlay({})
    invoke('set_playlist_tempo', { val: tempo as number })
  }, [tempo])

  return (
    <div className="min-h-screen text-neutral-200">
      <nav className="w-full">
        <ul className="flex flex-row space-x-3 place-items-center">
          <li aria-label="transport control">
            <button
              type="button"
              aria-label="Play/Pause"
              onClick={togglePlay}
              className="p-1.5 bg-neutral-800 border border-neutral-700/50 rounded-md hover:bg-neutral-700"
            >
              {playing ? <>Pause</> : <>Play</>}
            </button>
          </li>
          <li>
            <div className="flex flex-row space-x-1.5 place-items-center">
              <label htmlFor="tempo-control" className="text-sm text-neutral-400">
                Tempo:
              </label>
              <input
                id="tempo-control"
                name="tempo"
                type="number"
                value={tempo}
                onInput={(e) => {
                  setTempo(parseFloat(e.currentTarget.value))
                }}
                className="border border-neutral-700/50 bg-neutral-800 px-1 py-0.5 rounded-md w-14 focus:bg-neutral-700"
              />
            </div>
          </li>
        </ul>
      </nav>

      {playing && (
        <div
          style={{ display: 'flex', flexDirection: 'column', width: '200px', marginLeft: 'auto', marginRight: 'auto' }}
        >
          <span>Playlist runtime:</span>{' '}
          <span style={{ display: 'flex', flexDirection: 'row' }}>
            <span style={{ width: '100px', left: '0' }}>{runtime}</span> <span style={{ left: '100%' }}>ms</span>
          </span>
        </div>
      )}
    </div>
  )
}

export default App
