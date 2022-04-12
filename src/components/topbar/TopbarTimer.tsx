import React from 'react'
import {
  selectPlaylistPlaying,
  fetchPlaylistRuntime,
  selectPlaylistRuntime,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import styles from './styles.module.scss'

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
    <div className={`${styles.TopbarTimer} rounded`}>
      <span className={`${styles.TopbarTimerLabel} text-gray-400 text-xs text-left`}>
        Runtime:
      </span>
      {playing && <span className="text-gray-600 text-sm text-left">
        {runtime}
      </span>}
    </div>
  )
}

export default PlayPauseButton
