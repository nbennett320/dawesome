import React from 'react'
import {
  selectPlaylistPlaying,
  fetchPlaylistRuntime,
  selectPlaylistRuntime,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

const PlayPauseButton = () => {
  const timer: { current: NodeJS.Timer | null } = React.useRef(null)
  const runtime = useAppSelector(selectPlaylistRuntime)
  const playing = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    if (playing) {
      timer.current = setInterval(() => {
        dispatch(fetchPlaylistRuntime())
      }, 100)
    } else {
      clearInterval(timer.current as NodeJS.Timer)
    }
  }, [playing])

  return (
    <div>
      <span>Runtime: </span>
      {playing && <span>{runtime}</span>}
    </div>
  )
}

export default PlayPauseButton
