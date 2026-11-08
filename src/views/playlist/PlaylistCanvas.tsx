import React from 'react'
import { ReactP5Wrapper } from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api'
import { 
  addToPlaylist,
  moveNodeInPlaylist,
  removeFromPlaylist,
  selectPlaylistItems,
  selectPlaylistPlaying,
  selectPlaylistTempo,
  selectPlaylistUI
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { PlaylistWindow, PlaylistItem } from '../../types/playlist'
import { Renderer } from './sketch'
import { onSampleDrag } from '../../state/sampleDrag'
import SampleDragGhost from './SampleDragGhost'

const playlistRenderer = new Renderer()

const PlaylistCanvas = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [height, setHeight] = React.useState<number>()
  const [width, setWidth] = React.useState<number>()
  const [limit, setLimit] = React.useState<number>(16)
  const [beatsDisplayed, setBeatsDisplayed] = React.useState<number>(16)
  const [duration, setDuration] = React.useState<number>(120)
  const [trackCount, setTrackCount] = React.useState<number>(5)
  const [playlistWindow, setPlaylistWindow] = React.useState<PlaylistWindow>()

  const dispatch = useAppDispatch()
  const playing = useAppSelector(selectPlaylistPlaying)
  const tempo = useAppSelector(selectPlaylistTempo)
  const playlistTrackBoxRef = React.useRef<HTMLDivElement>(null)
  const items = useAppSelector(selectPlaylistItems)

  // samples dragged in from the sidebar: show the clip under the mouse while
  // it's over the playlist, and add it where the preview showed on release
  React.useEffect(() => onSampleDrag(({ type, path, clientX, clientY }) => {
    const el = ref.current
    if(!el) return

    // the element actually under the mouse, so other panes covering the
    // playlist's box don't count as being over it
    const over = el.contains(document.elementFromPoint(clientX, clientY))
    const { left, top, right, bottom } = el.getBoundingClientRect()
    const x = clientX - left
    const y = clientY - top

    if(type === 'move' && over) {
      playlistRenderer.setSampleDrag(path, x, y, top)
      return
    }

    playlistRenderer.clearSampleDrag()

    if(type === 'drop' && over) {
      const { beat, trackNumber } = playlistRenderer.dropTarget(x, y)

      dispatch(addToPlaylist(path, trackNumber, beat, {
        x: clientX,
        y: clientY,
        left,
        top,
        right,
        bottom,
      }))
    }
  }), [])

  console.log("items", items)

  // fit the canvas to the playlist's box whenever it changes size: the
  // window resizing, the sidebar toggling, a split pane's divider moving,
  // or the playlist's tab being shown again after another tab was selected
  React.useEffect(() => {
    const el = ref.current
    if(!el) return undefined

    const fit = () => {
      const { height: h, width: w } = el.getBoundingClientRect()

      // hidden behind another tab, keep the last size
      if(!w || !h) return

      setHeight(h)
      setWidth(w)
      playlistRenderer.setHeight(h)
      playlistRenderer.setWidth(w)
    }

    fit()
    const observer = new ResizeObserver(fit)
    observer.observe(el)

    return () => observer.disconnect()
  }, [])

  React.useEffect(() => {
    const getPlaylistData = async () => {
      const [
        maxPlaylistBeats,
        maxBeatsDisplayed,
        maxPlaylistDuration,
        calculatedTrackCount
      ] = await invoke<[number, number, number, number]>('get_playlist_data', {})
      setLimit(maxPlaylistBeats)
      setBeatsDisplayed(maxBeatsDisplayed)
      setTrackCount(calculatedTrackCount)
      setDuration(maxPlaylistDuration)
    }

    getPlaylistData()
  }, [])

  const handleItemDrop = (pw: PlaylistWindow) => {
    setPlaylistWindow(pw)
  }

  const handleNodeRightClick = async (id: number) => {
    dispatch(removeFromPlaylist(id)) 
  }

  // a clip already in the playlist was dragged and dropped. the renderer
  // works out the beat (accounting for zoom, pan, and where the clip was
  // grabbed) and the track from the mouse position
  const handleNodeMove = (
    item: PlaylistItem,
    dropBeat: number,
    trackNumber: number,
  ) => {
    dispatch(moveNodeInPlaylist(
      item.id,
      item.path,
      trackNumber,
      dropBeat,
      // keep the vertical offset the clip was first drawn with
      item.pixelOffset,
    ))
  }

  return (
    <div 
      ref={ref}
      className='w-full h-full'
      style={{ minHeight: '100vh' }}
    >
      {ref.current && <div>
        <ReactP5Wrapper
          sketch={playlistRenderer.sketch}
          style={{ height: '100%' }}
          height={height}
          width={width}
          playing={playing}
          tempo={tempo}
          beatsDisplayed={beatsDisplayed}
          maxPlaylistBeats={limit}
          duration={duration}
          trackCount={trackCount}
          playlistObjects={items}
          onItemDrop={handleItemDrop}
          onNodeRightClick={handleNodeRightClick}
          onNodeMove={handleNodeMove}
        />
      </div>}
      <SampleDragGhost playlistRef={ref} />
    </div>
  )
}

export default PlaylistCanvas
