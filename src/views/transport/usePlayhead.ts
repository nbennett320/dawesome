import { useAppSelector } from '../../hooks/redux'
import { selectPlaylistPlaying, selectWind } from '../../state/slices/playlistSlice'
import { positionAt, windPosition } from './tape'
import { useTicker, useTransport } from './useTransport'

// where the playhead is now, in ms, moving with playback or a wind (it
// re-renders about every 50ms while either is going)
export const usePlayhead = () => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const wind = useAppSelector(selectWind)
  const { transport, refresh, assume } = useTransport()

  useTicker(playing || wind !== null)

  let position = 0
  if(transport) {
    position = wind ? windPosition(wind, performance.now(), transport) : positionAt(transport, Date.now())
  }

  return { transport, position, wind, playing, refresh, assume }
}
