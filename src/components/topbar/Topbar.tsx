import React from 'react'
import TopbarBase from './TopbarBase'
import ButtonArea from './ButtonArea'
import TempoInput from './TempoInput'
import SnapInput from './SnapInput'
import TransportCounter from './TransportCounter'
import styles from './styles.module.scss'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectPlaylistPlaying, syncPlaying } from '../../state/slices/playlistSlice'

// the engine stops by itself at the end of the song (with looping off), so
// check now and then while playing
const usePlaybackSync = () => {
  const playing = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    if(!playing) return undefined

    const timer = setInterval(() => dispatch(syncPlaying()), 500)
    return () => clearInterval(timer)
  }, [playing, dispatch])
}

const Topbar = () => {
  usePlaybackSync()

  return (
    <TopbarBase>
      <div className={`${styles.Topbar} items-center gap-5 py-1.5`}>
        <ButtonArea />
        <div className='flex flex-row items-center gap-4'>
          <TempoInput />
          <SnapInput />
        </div>
        <TransportCounter />
      </div>
    </TopbarBase>
  )
}

export default Topbar
