import React from 'react'
import {
  selectPlaylistPlaying,
  fetchPlaylistRuntime,
  fetchPlaylistBeatCount,
  selectPlaylistRuntime,
  selectPlaylistBeatCount,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import styles from './styles.module.scss'

const PlayPauseButton = () => {
  const timer: { current: NodeJS.Timer | null } = React.useRef(null)
  const runtime = useAppSelector(selectPlaylistRuntime)
  const beatCount = useAppSelector(selectPlaylistBeatCount)
  const playing = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    if (playing) {
      timer.current = setInterval(() => {
        dispatch(fetchPlaylistRuntime())
        dispatch(fetchPlaylistBeatCount())
      }, 100)
    } else {
      clearInterval(timer.current as NodeJS.Timer)
    }
  }, [playing])
  const textClasses = `${styles.TopbarTimerLabel} text-gray-400 text-xs text-left`

  return (
    <div className={`${styles.TopbarTimer} rounded`}>
      <div className='flex flex-row'>
        <span className={textClasses}>
          Runtime:&nbsp;
        </span>
        {playing ? <span className={textClasses}>
          {runtime}s
        </span> : <span className={textClasses}>-</span>}
      </div>
      <div className='flex flex-row'>
        <span className={textClasses}>
          Beat count:&nbsp;
        </span>
        {playing ? <span className={textClasses}>
          {beatCount}
        </span> : <span className={textClasses}>-</span>}
      </div>
    </div>
  )
}

export default PlayPauseButton
