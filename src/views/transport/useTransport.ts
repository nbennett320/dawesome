import React from 'react'
import { invoke } from '@tauri-apps/api/core'
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
const selectPatterns = (state: RootState) => state.sequencer.patterns

// settings are sent to the engine a little behind (see sendSettings in
// playlistSlice), so it's asked again once they've got there
const ASK_AGAIN_MS = 200

// the engine's transport: where the playhead is, and how to follow it while
// playing. fetched again whenever something it reports may have changed
export const useTransport = () => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const items = useAppSelector(selectPlaylistItems)
  const tempo = useAppSelector(selectPlaylistTempo)
  const loopEnabled = useAppSelector(selectLoopEnabled)
  // a clip's pitch changes how long it plays, so where the song ends
  const clipSettings = useAppSelector(selectClipSettings)
  // a pattern's length changes where its clips end
  const patterns = useAppSelector(selectPatterns)
  const revision = useAppSelector(selectTransportRevision)
  const [transport, setTransport] = React.useState<PlaylistTransport | null>(null)

  const again = React.useRef<ReturnType<typeof setTimeout> | null>(null)
  // the transport as last fetched
  const last = React.useRef<PlaylistTransport | null>(null)

  const refresh = React.useCallback(
    (): Promise<void> => invoke<PlaylistTransport>('get_playlist_transport', {})
      .then(next => {
        // the loop's length changed while playing: the engine takes it up on
        // its next beat (and counts the playhead from there), so ask again
        // once it has
        const before = last.current
        if(next.playing && before && before.loopBeats !== next.loopBeats) {
          if(again.current) clearTimeout(again.current)
          again.current = setTimeout(() => { refresh() }, next.beatIntervalMs + 250)
        }
        last.current = next
        setTransport(next)
      })
      // eslint-disable-next-line no-console
      .catch(e => console.error('could not get the transport:', e)),
    [],
  )

  React.useEffect(() => {
    refresh()
    const timer = setTimeout(refresh, ASK_AGAIN_MS)
    return () => clearTimeout(timer)
  }, [refresh, playing, items, tempo, loopEnabled, clipSettings, patterns, revision])

  React.useEffect(() => () => {
    if(again.current) clearTimeout(again.current)
  }, [])

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
