import React from 'react'
import { selectPlaylistUI } from '../../state/slices/playlistSlice'
import { useAppSelector } from '../../hooks/redux'
import styles from './styles.module.scss'

const zeroPad = (num: number, places: number): string => (num.toString()).padStart(places, '0')

const PlaylistTimeline = () => {
  const playlistUI = useAppSelector(selectPlaylistUI)
  const range = [...Array(playlistUI.maxPlaylistBeats).keys()].map(e => e + 1)

  return (
    <div className={`${styles.PlaylistTimelineContainer}`}>
      <div 
        style={{ width: `calc(${100 * playlistUI.displayRatio}% + 2.5px)` }}
        className={`${styles.PlaylistTimeline} bg-slate-300`}
      >
        {range.map(e => (
          <div 
            key={`timeline-segment-${e}`}
            id={`playlist-timeline-segment-${e}`}
            style={{ width: `calc(${100 * playlistUI.displayRatio / playlistUI.maxPlaylistBeats / 2}%)` }}
            className={`${styles.PlaylistTimelineSegment} border-l-slate-400`}
          >
            <span className={`${styles.PlaylistTimelineSegmentText} text-gray-600 text-xs font-mono`}>
              {e < 10 ? zeroPad(e, 1) : e}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}

export default PlaylistTimeline
