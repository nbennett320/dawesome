import React from 'react'
import { 
  selectPlaylistPlaying,
  PlayPausePlaylistState
} from '../../state/slices/playPausePlaylistSlice'
import {
  fetchPlaylistRuntime,
  selectPlaylistRuntime,
  PlaylistRuntimeState
} from '../../state/slices/playlistRuntimeSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

const PlayPauseButton = () => {
  const timer: { current: NodeJS.Timer | null } = React.useRef(null)
  const runtime = useAppSelector<PlaylistRuntimeState>(selectPlaylistRuntime) 
  const playing = useAppSelector<PlayPausePlaylistState>(selectPlaylistPlaying) 
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    console.log("use effect")
    if(playing.value) {
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
      {playing.value && <span>{runtime.value}</span>}
    </div>
  )
}

export default PlayPauseButton

