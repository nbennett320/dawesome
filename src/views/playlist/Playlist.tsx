import React from 'react'
import { invoke } from '@tauri-apps/api'
import { initPlaylistTimeline } from '../../state/slices/playlistSlice'
import { useAppDispatch } from '../../hooks/redux'
import PlaylistTimeline from './PlaylistTimeline'
import PlaylistGrid from './PlaylistGrid'
import PlaylistTrackContainer from './PlaylistTrackContainer'
import styles from './styles.module.scss'

const Playlist = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    // initialize the playlist viewport so we can
    // calculate the current zoom on the playlist view
    const initPlaylistWorkspace = async () => {
      if(ref?.current) {
        const { left, top, right, bottom } = ref.current.getBoundingClientRect()
        await invoke<void>('init_playlist_workspace', {
          minBoundX: left,
          minBoundY: top,
          maxBoundX: right,
          maxBoundY: bottom,
        })
      }
    }

    initPlaylistWorkspace().then(() => {
      dispatch(initPlaylistTimeline())
    })
  }, [])

  return (
    <div 
      ref={ref}
      className={`${styles.Playlist} h-full`}
    >
      <PlaylistTimeline />
      {/* <PlaylistGrid /> */}
      <PlaylistTrackContainer />
    </div>
  )
}

export default Playlist
