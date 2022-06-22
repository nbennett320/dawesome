import React from 'react'
import { invoke } from '@tauri-apps/api'
import PlaylistTimeline from './PlaylistTimeline'
import PlaylistTrack from './PlaylistTrack'
import styles from './styles.module.scss'

const Playlist = () => {
  const ref = React.useRef<HTMLDivElement>(null)

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

    initPlaylistWorkspace()
  }, [])

  return (
    <div 
      ref={ref}
      className={`${styles.Playlist} h-full`}
    >
      <PlaylistTimeline />
      <div className={styles.PlaylistTrackContainer}>
        {[0, 1, 2, 3].map((el, i) => (
          <PlaylistTrack 
            key={`playlist-track-${el}`}
            trackNumber={i}
          />
        ))}
      </div>
    </div>
  )
}

export default Playlist
