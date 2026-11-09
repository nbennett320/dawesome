import React from 'react'
import { useStore } from 'react-redux'
import { ReactP5Wrapper } from 'react-p5-wrapper'
import { 
  addToPlaylist,
  inspectClip,
  moveNodeInPlaylist,
  moveNodesInPlaylist,
  removeFromPlaylist,
  seekPlaylist,
  selectClipOverrides,
  selectPlaylistItems,
  selectPlaylistPlaying,
  selectPlaylistTempo,
  selectPlaylistUI,
  selectTransportRevision,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import { PlaylistWindow, PlaylistItem } from '../../types/playlist'
import { Renderer } from './sketch'
import { onSampleDrag } from '../../state/sampleDrag'
import SampleDragGhost from './SampleDragGhost'

const playlistRenderer = new Renderer()

const PlaylistCanvas = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [height, setHeight] = React.useState<number>()
  const [width, setWidth] = React.useState<number>()
  const [playlistWindow, setPlaylistWindow] = React.useState<PlaylistWindow>()

  const dispatch = useAppDispatch()
  const playing = useAppSelector(selectPlaylistPlaying)
  const transportRevision = useAppSelector(selectTransportRevision)
  const handleSeek = React.useCallback((beat: number) => { dispatch(seekPlaylist(beat)) }, [dispatch])
  const tempo = useAppSelector(selectPlaylistTempo)
  const playlistTrackBoxRef = React.useRef<HTMLDivElement>(null)
  const items = useAppSelector(selectPlaylistItems)
  // the playlist's length and tracks, from the engine. refreshed when the
  // playlist first shows and whenever a project is loaded (see initPlaylist)
  const {
    maxPlaylistBeats: limit,
    maxBeatsDisplayed: beatsDisplayed,
    maxPlaylistDuration: duration,
    trackCount,
  } = useAppSelector(selectPlaylistUI)

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

  // show clips' played lengths and modified dots as their settings change.
  // read from the store directly rather than as props: a prop update
  // rebuilds every clip, which would flicker while dragging a control
  const store = useStore<RootState>()
  React.useEffect(() => {
    let last: RootState['playlist'] | null = null

    const apply = () => {
      const { playlist } = store.getState()
      const changed = !last
        || playlist.clipSettings !== last.clipSettings
        || playlist.clipSampleDurations !== last.clipSampleDurations
      last = playlist

      if(changed) playlistRenderer.setClipOverrides(selectClipOverrides(store.getState()))
    }

    apply()
    return store.subscribe(apply)
  }, [store])

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

  // stable across renders: the p5 wrapper rebuilds every clip when any of
  // its props change, including these handlers
  const handleItemDrop = React.useCallback((pw: PlaylistWindow) => {
    setPlaylistWindow(pw)
  }, [])

  const handleNodeRightClick = React.useCallback((id: number) => {
    dispatch(removeFromPlaylist(id))
  }, [dispatch])

  const handleNodeDoubleClick = React.useCallback((id: number) => {
    dispatch(inspectClip(id))
  }, [dispatch])

  // a clip already in the playlist was dragged and dropped. the renderer
  // works out the beat (accounting for zoom, pan, and where the clip was
  // grabbed) and the track from the mouse position
  const handleNodeMove = React.useCallback((
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
  }, [dispatch])

  // a selection was dragged: the grabbed clip to dropBeat on trackNumber,
  // and the rest along with it
  const handleNodesMove = React.useCallback((
    ids: number[],
    anchorId: number,
    dropBeat: number,
    trackNumber: number,
  ) => {
    dispatch(moveNodesInPlaylist(ids, anchorId, dropBeat, trackNumber))
  }, [dispatch])

  return (
    <div 
      ref={ref}
      // fills the pane's body exactly, so the canvas is the visible height and
      // the tracks scroll inside it
      className='absolute inset-0 overflow-hidden'
    >
      {/* only once the box has been measured (see fit) */}
      {ref.current && !!width && !!height && <div>
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
          onNodesMove={handleNodesMove}
          onNodeDoubleClick={handleNodeDoubleClick}
          transportRevision={transportRevision}
          onSeek={handleSeek}
        />
      </div>}
      <SampleDragGhost playlistRef={ref} />
    </div>
  )
}

// memo: it takes no props, so pane focus and tab changes above it don't
// re-render it (and rebuild every clip)
export default React.memo(PlaylistCanvas)
