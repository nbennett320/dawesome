import React from 'react'
import { ReactP5Wrapper } from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api'
import { useDrop } from 'react-dnd'
import { 
  addToPlaylist,
  moveNodeInPlaylist,
  removeFromPlaylist,
  selectPlaylistItems,
  selectPlaylistPlaying,
  selectPlaylistUI
} from '../../state/slices/playlistSlice'
import { selectSidebar, selectWindowPane } from '../../state/slices/windowSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { PlaylistWindow, PlaylistTypes, PlaylistItemPixelOffset, PlaylistItem } from '../../types/playlist'
import { Renderer } from './sketch'

const playlistRenderer = new Renderer()

const PlaylistCanvas = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [height, setHeight] = React.useState<number>()
  const [width, setWidth] = React.useState<number>()
  const [limit, setLimit] = React.useState<number>(16)
  const [duration, setDuration] = React.useState<number>(120)
  const [trackCount, setTrackCount] = React.useState<number>(5)
  const [playlistWindow, setPlaylistWindow] = React.useState<PlaylistWindow>()

  const dispatch = useAppDispatch()
  const playing = useAppSelector(selectPlaylistPlaying)
  const sidebar = useAppSelector(selectSidebar)
  const windowPane = useAppDispatch(selectWindowPane)
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
            const canvas = ref.current.firstChild?.firstChild?.firstChild
            const dropData = {
              x,
              y,
              left,
              top,
              right,
              bottom,
            } as PlaylistItemPixelOffset
            const trackNumber = playlistRenderer.calculateTrackNumber(dropData)
            console.log("dropped on ref:", ref.current, canvas)

            dispatch(addToPlaylist(
              item.name as string, 
              trackNumber,
              {
                x,
                y,
                left,
                top,
                right,
                bottom,
              },
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

  const items = useAppSelector(selectPlaylistItems)

  console.log("items", items)

  const updateWidthAndHeight = () => {
    setWidth(ref.current?.clientWidth)
    setHeight(ref.current?.clientHeight)
  }

  React.useEffect(() => {
    // set width when ref or sidebar update
    if(ref.current) {
      const { height: h, width: w } = ref.current.getBoundingClientRect()
      setHeight(h)
      setWidth(w)
      playlistRenderer.setHeight(h)
      playlistRenderer.setWidth(w)
      if(!h) {
        console.log("setting it bigger")
        playlistRenderer.setHeight(1000)
      }
      console.log("the height:", h)
    }
  }, [ref.current, sidebar, windowPane])

  React.useEffect(() => {
    // resize when sizebar changes
    if(ref.current) {
      const { height: h, width: w } = ref.current.getBoundingClientRect()
      setHeight(h)
      setWidth(w)
      playlistRenderer.setHeight(h)
      playlistRenderer.setWidth(w)
    }
  }, [sidebar])

  React.useEffect(() => {
    const getPlaylistData = async () => {
      const [
        maxPlaylistBeats,
        maxBeatsDisplayed,
        maxPlaylistDuration,
        calculatedTrackCount
      ] = await invoke<[number, number, number, number]>('get_playlist_data', {})
      setLimit(maxPlaylistBeats)
      setTrackCount(calculatedTrackCount)
      setDuration(maxPlaylistDuration)
    }

    window.addEventListener('resize', updateWidthAndHeight)

    getPlaylistData()

    return () => window.removeEventListener('resize', updateWidthAndHeight)
  }, [])

  const handleItemDrop = (pw: PlaylistWindow) => {
    setPlaylistWindow(pw)
  }

  const handleNodeRightClick = async (id: number) => {
    dispatch(removeFromPlaylist(id)) 
  }

  const handleNodeMove = (
    item: PlaylistItem,
    dropCoords: { x: number, y: number },
  ) => {
    console.log("node move")
    // handle drag and drop from nodes already existent in the playlist
    if(dropCoords && ref.current) { 
      const { x, y } = dropCoords
      const { left, top, right, bottom } = ref.current.getBoundingClientRect()
      const dropData = {
        x,
        y,
        left,
        top,
        right,
        bottom,
      } as PlaylistItemPixelOffset
      const trackNumber = playlistRenderer.calculateTrackNumber(dropData)

      dispatch(moveNodeInPlaylist(
        item.id,
        item.path as string, 
        trackNumber,
        {
          x,
          y,
          left,
          top,
          right,
          bottom,
        },
      ))
    } else {
      // eslint-disable-next-line no-console
      console.error(
        "Error moving node: no playlist track ref, or null drop coordinates:", 
        item, 
        dropCoords, 
        ref.current
      )
    }
  }

  return (
    <div 
      ref={ref}
      className='w-full h-full'
      style={{ minHeight: '100vh' }}
    >
      {ref.current && <div ref={dropRef}>
        <ReactP5Wrapper
          sketch={playlistRenderer.sketch}
          style={{ height: '100%' }}
          height={height}
          width={width}
          playing={playing}
          maxPlaylistBeats={limit}
          duration={duration}
          trackCount={trackCount}
          playlistObjects={items}
          onItemDrop={handleItemDrop}
          onNodeRightClick={handleNodeRightClick}
          onNodeMove={handleNodeMove}
        />
      </div>}
    </div>
  )
}

export default PlaylistCanvas
