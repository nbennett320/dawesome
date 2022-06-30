import React from 'react'
import PlaylistTrack from './PlaylistTrack'
import styles from './styles.module.scss'

const PlaylistTrackContainer = () => {
  const [tracks, setTracks] = React.useState(4)
  const range = [...Array(tracks).keys()]

  return (
    <div className={styles.PlaylistTrackContainer}>
      {range.map((el, i) => (
        <PlaylistTrack 
          key={`playlist-track-${el}`}
          trackNumber={i}
        />
      ))}
    </div>
  )
}

export default PlaylistTrackContainer
