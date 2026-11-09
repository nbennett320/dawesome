import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useAppSelector } from '../../hooks/redux'
import {
  selectLoopEnabled,
  selectPlaylistItems,
  selectPlaylistPlaying,
  selectPlaylistTempo,
  selectTransportRevision,
} from '../../state/slices/playlistSlice'
import { RootState } from '../../state/store'
import { PlaylistTransport } from '../../types/transport'

const selectClipSettings = (state: RootState) => state.playlist.clipSettings

// the engine's transport: where the playhead is, and how to follow it while
// playing. fetched again whenever something it reports may have changed
export const useTransport = () => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const items = useAppSelector(selectPlaylistItems)
  const tempo = useAppSelector(selectPlaylistTempo)
  const loopEnabled = useAppSelector(selectLoopEnabled)
  // a clip's pitch changes how long it plays, so where the song ends
  const clipSettings = useAppSelector(selectClipSettings)
  const revision = useAppSelector(selectTransportRevision)
  const [transport, setTransport] = React.useState<PlaylistTransport | null>(null)

  const refresh = React.useCallback(
    () => invoke<PlaylistTransport>('get_playlist_transport', {})
      .then(setTransport)
      // eslint-disable-next-line no-console
      .catch(e => console.error('could not get the transport:', e)),
    [],
  )

  React.useEffect(() => {
    refresh()
  }, [refresh, playing, items, tempo, loopEnabled, clipSettings, revision])

  // show the playhead somewhere straight away, before the engine confirms it
  const assume = React.useCallback((positionMs: number) => {
    setTransport(t => t && { ...t, playing: false, positionMs })
  }, [])

  return { transport, refresh, assume }
}

// re-render about every `everyMs` while `active`, for things that move with
// the playhead
export const useTicker = (active: boolean, everyMs = 50) => {
  const [, setTick] = React.useState(0)

  React.useEffect(() => {
    if(!active) return undefined

    let id = 0
    let last = 0
    const step = (now: number) => {
      if(now - last >= everyMs) {
        last = now
        setTick(n => n + 1)
      }
      id = requestAnimationFrame(step)
    }

    id = requestAnimationFrame(step)
    return () => cancelAnimationFrame(id)
  }, [active, everyMs])
}
