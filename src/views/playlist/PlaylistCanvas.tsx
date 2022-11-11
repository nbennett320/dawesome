import React from 'react'
import { ReactP5Wrapper } from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api'
import { useDrop } from 'react-dnd'
import { 
  addToPlaylist,
  moveNodeInPlaylist,
  removeFromPlaylist,
  selectPlaylistItems,
  selectPlaylistUI
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { PlaylistTypes } from '../../types/playlist'
import sketch from './sketch'

const PlaylistCanvas = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [height, setHeight] = React.useState<number>()
  const [width, setWidth] = React.useState<number>()
  const [limit, setLimit] = React.useState<number>(16)
  const [ratio, setRatio] = React.useState<number>(1)
  const range = [...Array(limit).keys()].map(e => e + 1)

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
      console.log("dropped: ", item)
      switch(itemType) {
        case PlaylistTypes.SidebarSampleItem: 
          // handle samples dropped from the sidebar
          if(dropCoords && ref.current) { 
            const { x, y } = dropCoords
            const { left, top, right, bottom } = ref.current.getBoundingClientRect()
            const pixelOffset = x - left

            dispatch(addToPlaylist(
              item.name as string, 
              1,
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
          if(dropCoords && ref.current) { 
            const { x, y } = dropCoords
            const { left, top, right, bottom } = ref.current.getBoundingClientRect()
            const pixelOffset = x - left

            dispatch(moveNodeInPlaylist(
              item.id, 
              item.name as string,
              2,
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
              ref.current
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

  const items = useAppSelector(selectPlaylistItems)
    .filter(item => item.trackNumber === 1)

  console.log("items", items)

  console.log("canDrop, isOver", canDrop, isOver)

  React.useEffect(() => {
    if(ref.current) {
      const { height: h, width: w } = ref.current.getBoundingClientRect()
      setHeight(h)
      setWidth(w)
    }
  }, [ref.current])

  React.useEffect(() => {
    const getPlaylistTimeline = async () => {
      const [maxPlaylistBeats, maxBeatsDisplayed, displayRatio] = 
        await invoke<[number, number, number]>('get_playlist_timeline', {})
      setLimit(maxPlaylistBeats)
      setRatio(displayRatio)
    }

    getPlaylistTimeline()
  }, [])

  return (
    <div 
      ref={ref}
      className='w-full h-full'
    >
      {ref.current && <div ref={dropRef}>
        <ReactP5Wrapper 
          sketch={sketch}
          height={height}
          width={width}
          maxPlaylistBeats={limit}
          playlistObjects={items}
        />
      </div>}
    </div>
  )
}

export default PlaylistCanvas
