import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrop } from 'react-dnd'
import { 
  addToPlaylist,
  removeFromPlaylist,
  selectPlaylistItems
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import PlaylistItem from './PlaylistItem'
import { PlaylistTypes } from '../../types/playlist'
import styles from './styles.module.scss'

interface Props {
  trackNumber: number
}

const PlaylistTrack = (props: Props) => {
  const dispatch = useAppDispatch()
  const playlistTrackBoxRef = React.useRef<HTMLDivElement>(null)
  const [{ canDrop, isOver }, dropRef] = useDrop(() => ({
    accept: PlaylistTypes.PlaylistTrackItem,
    drop: async (item: Record<string, any>, monitor) => {
      const dropCoords = monitor.getClientOffset()
      if(dropCoords && playlistTrackBoxRef.current) { 
        const { x, y } = dropCoords
        const { left, top, right, bottom } = playlistTrackBoxRef.current.getBoundingClientRect()
        const offset = await invoke<number>('get_playlist_sample_offset', {
          dropX: x,
          dropY: y,
          minBoundX: left,
          minBoundY: top,
          maxBoundX: right,
          maxBoundY: bottom,
        })

        dispatch(addToPlaylist(
          item.name as string, 
          offset, 
          props.trackNumber,
          x,
        ))
      } else {
        // eslint-disable-next-line no-console
        console.error(
          "Error dropping: no playlist track ref, or null drop coordinates:", 
          item, 
          dropCoords, 
          playlistTrackBoxRef.current
        )
      }
    },
    collect: (monitor) => ({
      isOver: monitor.isOver(),
      canDrop: monitor.canDrop(),
    }),
  }))

  const items = useAppSelector(selectPlaylistItems)
    .filter(item => item.trackNumber === props.trackNumber)

  const isActive = canDrop && isOver

  const handleItemRightClick = async (id: number) => {
    dispatch(removeFromPlaylist(id)) 
  }

  return (
    <div 
      ref={dropRef}
      data-testid='playlist-track'
      className={`${styles.PlaylistTrackWrapper} border-slate-400`}
    >
      <div
        ref={playlistTrackBoxRef} 
        className={`${styles.PlaylistTrack} h-full`}
      >
        {items.map((e, i) => (
          <PlaylistItem 
            onRightClick={handleItemRightClick}
            key={`${e}-${i as unknown as string}`}
            id={e.id}
            value={e.path}
            pixelOffset={e.pixelOffset}
          />
        ))}
      </div>
    </div>
  )
}

export default PlaylistTrack
