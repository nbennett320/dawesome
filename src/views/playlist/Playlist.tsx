import React from 'react'
import PlaylistTimeline from './PlaylistTimeline'
import styles from './styles.module.scss'

const Playlist = () => (
  <div className={`${styles.Playlist}`}>
    <PlaylistTimeline />
  </div>
)

export default Playlist
