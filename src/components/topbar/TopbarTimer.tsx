import React from 'react'
import {
  selectPlaylistPlaying,
  fetchPlaylistRuntime,
  fetchPlaylistBeatCount,
  selectPlaylistRuntime,
  selectPlaylistBeatCount,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

// the runtime and beat count, on a soft display
const TopbarTimer = () => {
  const timer = React.useRef<ReturnType<typeof setInterval> | null>(null)
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
    } else if (timer.current) {
      clearInterval(timer.current)
    }
  }, [playing])

  return (
    <div className='ft-lcd flex flex-row items-baseline gap-2 text-sm py-0.5' aria-live='off'>
      <span className='ft-label text-[9px]'>Time</span>
      <span className='text-right min-w-[5.5rem]'>{playing ? `${runtime}s` : '--'}</span>
      <span className='ft-label text-[9px]'>Beat</span>
      <span className='text-right min-w-[1.5rem]'>{playing ? beatCount : '--'}</span>
    </div>
  )
}

export default TopbarTimer
