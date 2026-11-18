import { useAppSelector } from '../../hooks/redux'
import { selectPlaylistPlaying, selectWind } from '../../state/slices/playlistSlice'
import { positionAt, windPosition } from './tape'
import { useTicker, useTransport } from './useTransport'

// where the playhead is now, in ms, moving with playback or a wind (it
// re-renders about every `everyMs` while either is going: 50ms suits a
// counter; a moving line wants every frame, 0)
export const usePlayhead = (everyMs = 50) => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const wind = useAppSelector(selectWind)
  const { transport, refresh, assume } = useTransport()

  useTicker(playing || wind !== null, everyMs)

  let position = 0
  if(transport) {
    position = wind ? windPosition(wind, performance.now(), transport) : positionAt(transport, Date.now())
  }

  return { transport, position, wind, playing, refresh, assume }
}
