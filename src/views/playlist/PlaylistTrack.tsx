import React from 'react'
import { useDrop } from 'react-dnd'
import { 
  addToPlaylist,
  moveNodeInPlaylist,
  removeFromPlaylist,
  selectPlaylistItems,
  selectPlaylistUI
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
    accept: [
      PlaylistTypes.SidebarSampleItem, 
      PlaylistTypes.PlaylistTrackItem,
    ],
    drop: async (item: Record<string, any>, monitor) => {
      const dropCoords = monitor.getClientOffset()
      const itemType = monitor.getItemType()
      switch(itemType) {
        case PlaylistTypes.SidebarSampleItem: 
          // handle samples dropped from the sidebar
          if(dropCoords && playlistTrackBoxRef.current) { 
            const { x, y } = dropCoords
            const { left, top, right, bottom } = playlistTrackBoxRef.current.getBoundingClientRect()
            const pixelOffset = x - left

            dispatch(addToPlaylist(
              item.name as string, 
              props.trackNumber,
              x,
              y,
              pixelOffset,
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
          break
        case PlaylistTypes.PlaylistTrackItem:
          // handle drag and drop from nodes already existent in the playlist
          if(dropCoords && playlistTrackBoxRef.current) { 
            const { x, y } = dropCoords
            const { left, top, right, bottom } = playlistTrackBoxRef.current.getBoundingClientRect()
            const pixelOffset = x - left

            dispatch(moveNodeInPlaylist(
              item.id, 
              item.name as string,
              props.trackNumber,
              x,
              y,
              pixelOffset,
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
          break
        default:
          // unrecognised drop
          // eslint-disable-next-line no-console
          console.error(
            "Error dropping: unknown type for the item being dropped:", 
            item, 
            monitor.getItemType()
          )
      }
    },
    collect: (monitor) => ({
      isOver: monitor.isOver(),
      canDrop: monitor.canDrop(),
    }),
  }))

  const playlistUI = useAppSelector(selectPlaylistUI)
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
      style={{ 
        width: `${100 * playlistUI.displayRatio}%`,
        backgroundImage: `
          repeating-linear-gradient(
            90deg, 
            rgb(203 213 225 / var(--tw-bg-opacity)) 0 1px,
            transparent 1px 100%
          )`,
        backgroundSize: `${100 * playlistUI.displayRatio / playlistUI.maxPlaylistBeats / 2}% ${101}px`,
      }}
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
