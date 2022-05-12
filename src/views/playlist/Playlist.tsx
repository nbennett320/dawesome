import React from 'react'
import PlaylistTimeline from './PlaylistTimeline'
import PlaylistTrack from './PlaylistTrack'
import styles from './styles.module.scss'

const Playlist = () => (
  <div className={`${styles.Playlist} h-full`}>
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

export default Playlist
