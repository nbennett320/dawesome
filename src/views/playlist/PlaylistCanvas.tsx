import React from 'react'
import { useStore } from 'react-redux'
import { ReactP5Wrapper } from 'react-p5-wrapper'
import { 
  addPatternToPlaylist,
  addToPlaylist,
  inspectClip,
  moveNodeInPlaylist,
  moveNodesInPlaylist,
  removeFromPlaylist,
  removeManyFromPlaylist,
  seekPlaylist,
  selectClipOverrides,
  selectLoopEnabled,
  selectPlaylistItems,
  selectPlaylistPlaying,
  selectPlaylistTempo,
  selectPlaylistUI,
  selectTransportRevision,
  setClipWindows,
  stretchClips,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import { PlaylistWindow, PlaylistItem } from '../../types/playlist'
import { Renderer, StretchEdit, WindowEdit } from './sketch'
import { isPlaceable, onSampleDrag } from '../../state/sampleDrag'
import { OWN_DRAG_PREVIEW } from '../../components/DragGhost'
import { selectFocusedView } from '../../state/slices/windowSlice'
import { View } from '../../types/ui'
import { isDeleteKey, isExplodeKey, isTypingTarget } from '../../util/keys'
import { explodePatternClips, showPattern } from '../../state/slices/sequencerSlice'
import ClipMenu from './ClipMenu'

const playlistRenderer = new Renderer()

// a clip's right-click menu: where it opens, relative to the playlist's box
interface OpenMenu {
  id: number
  x: number
  y: number
}

// the right-click menu, with its own state, so opening it doesn't re-render
// the canvas. `opener` is given the function that opens it
const ClipMenuHost = ({ opener }: { opener: React.MutableRefObject<((m: OpenMenu) => void) | null> }) => {
  const dispatch = useAppDispatch()
  const [menu, setMenu] = React.useState<OpenMenu | null>(null)
  const item = useAppSelector(selectPlaylistItems).find(i => i.id === menu?.id)
  const close = React.useCallback(() => setMenu(null), [])

  React.useEffect(() => {
    // eslint-disable-next-line no-param-reassign
    opener.current = setMenu
    // eslint-disable-next-line no-param-reassign
    return () => { opener.current = null }
  }, [opener])

  if(!menu || !item) return null

  const { patternId } = item
  const items = [
    ...(patternId === undefined ? [] : [{
      label: 'Explode into tracks',
      shortcut: '⇧E',
      onSelect: () => { dispatch(explodePatternClips([item.id])) },
    }, {
      label: 'Open in Sequencer',
      onSelect: () => { dispatch(showPattern(patternId)) },
    }]),
    {
      label: 'Delete',
      shortcut: '⌫',
      danger: true,
      onSelect: () => {
        // eslint-disable-next-line no-console
        dispatch(removeFromPlaylist(item.id)).catch(e => console.error('could not delete the clip:', e))
      },
    },
  ]

  return <ClipMenu x={menu.x} y={menu.y} items={items} onClose={close} />
}

const PlaylistCanvas = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [height, setHeight] = React.useState<number>()
  const [width, setWidth] = React.useState<number>()
  const [playlistWindow, setPlaylistWindow] = React.useState<PlaylistWindow>()

  const dispatch = useAppDispatch()
  const playing = useAppSelector(selectPlaylistPlaying)
  const transportRevision = useAppSelector(selectTransportRevision)
  const loopEnabled = useAppSelector(selectLoopEnabled)
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

  // samples dragged in from the sidebar, and patterns from the Sequencer:
  // show the clip under the mouse while it's over the playlist, and add it
  // where the preview showed on release
  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    const el = ref.current
    // folders only move within the sidebar
    if(!el || !isPlaceable(item)) return

    // the element actually under the mouse, so other panes covering the
    // playlist's box don't count as being over it
    const over = el.contains(document.elementFromPoint(clientX, clientY))
    const { left, top, right, bottom } = el.getBoundingClientRect()
    const x = clientX - left
    const y = clientY - top

    if(type === 'move' && over) {
      playlistRenderer.setSampleDrag(item, x, y, top)
      return
    }

    playlistRenderer.clearSampleDrag()

    if(type === 'drop' && over) {
      const { beat, trackNumber } = playlistRenderer.dropTarget(x, y)

      if(item.kind === 'pattern') {
        dispatch(addPatternToPlaylist(item.patternId, trackNumber, beat))
          // eslint-disable-next-line no-console
          .catch(e => console.error('could not place the pattern:', e))
        return
      }

      dispatch(addToPlaylist(item.path, trackNumber, beat, {
        x: clientX,
        y: clientY,
        left,
        top,
        right,
        bottom,
      }))
    }
  }), [])

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
        || playlist.clipWindows !== last.clipWindows
      // a project was opened or started: back to its first bars
      const opened = !!last && playlist.viewRevision !== last.viewRevision
      // an undo or redo changed some clips, which it selects
      const travelled = !!last && playlist.travelled !== last.travelled
      last = playlist

      if(opened) playlistRenderer.resetView()
      if(travelled) playlistRenderer.select(playlist.travelled.ids)
      if(changed) playlistRenderer.setClipOverrides(selectClipOverrides(store.getState()))
      // trimmed clips show the part of their sample or pattern they play
      playlistRenderer.setClipWindows(playlist.clipWindows)
      // pattern clips draw their pattern as it's edited
      playlistRenderer.setSequencer(store.getState().sequencer)
    }

    apply()
    return store.subscribe(apply)
  }, [store])

  // Delete or Backspace removes the selected clips, while the playlist is
  // the focused pane's tab and nothing that takes typing has the key
  React.useEffect(() => {
    const onKeyDown = (ev: KeyboardEvent) => {
      if(!isDeleteKey(ev) || ev.repeat || isTypingTarget(ev.target)) return
      if(selectFocusedView(store.getState()) !== View.Playlist) return

      const ids = playlistRenderer.takeSelection()
      if(!ids.length) return
      // Backspace would otherwise navigate back in some webviews
      ev.preventDefault()
      dispatch(removeManyFromPlaylist(ids))
        // eslint-disable-next-line no-console
        .catch(e => console.error('could not delete the clips:', e))
    }

    // Shift+E explodes the selected pattern clips (sample clips in the
    // selection are left alone)
    const onExplodeKey = (ev: KeyboardEvent) => {
      if(!isExplodeKey(ev) || ev.repeat || isTypingTarget(ev.target)) return
      if(selectFocusedView(store.getState()) !== View.Playlist) return
      // (not while clips are being dragged, trimmed or boxed)
      if(playlistRenderer.isDragging()) return

      const patternClips = new Set(store.getState().playlist.playlistItems
        .filter(item => item.patternId !== undefined)
        .map(item => item.id))
      const ids = [...playlistRenderer.selectedIds].filter(id => patternClips.has(id))
      if(!ids.length) return
      ev.preventDefault()
      dispatch(explodePatternClips(ids))
    }

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keydown', onExplodeKey)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keydown', onExplodeKey)
    }
  }, [store, dispatch])

  // fit the canvas to the playlist's box whenever it changes size (or is
  // hidden, which stops it drawing): the
  // window resizing, the sidebar toggling, a split pane's divider moving,
  // or the playlist's tab being shown again after another tab was selected
  React.useEffect(() => {
    const el = ref.current
    if(!el) return undefined

    const fit = () => {
      const { height: h, width: w } = el.getBoundingClientRect()

      // hidden behind another tab: keep the last size, and stop drawing
      // until it's shown again
      if(!w || !h) {
        playlistRenderer.setVisible(false)
        return
      }

      playlistRenderer.setVisible(true)
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

  // stable across renders, so the p5 wrapper isn't sent new props (and the
  // renderer doesn't look for changes) every time this re-renders
  const handleItemDrop = React.useCallback((pw: PlaylistWindow) => {
    setPlaylistWindow(pw)
  }, [])

  // right-clicking a clip opens its menu where the mouse is
  const openMenu = React.useRef<((m: OpenMenu) => void) | null>(null)
  const handleNodeRightClick = React.useCallback((id: number, clientX: number, clientY: number) => {
    const box = ref.current?.getBoundingClientRect()
    if(box) openMenu.current?.({ id, x: clientX - box.left, y: clientY - box.top })
  }, [])

  // a sample clip opens in the Sample tab, a pattern clip in the Sequencer
  const handleNodeDoubleClick = React.useCallback((id: number) => {
    const item = store.getState().playlist.playlistItems.find(i => i.id === id)
    if(item?.patternId !== undefined) dispatch(showPattern(item.patternId))
    else dispatch(inspectClip(id))
  }, [dispatch, store])

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

  // clips' edges were dragged: trim them all, as one step to undo
  const handleClipsTrim = React.useCallback(
    (edits: WindowEdit[]) => dispatch(setClipWindows(edits)),
    [dispatch],
  )

  // and with shift held: stretch them all, as one step to undo
  const handleClipsStretch = React.useCallback(
    (edits: StretchEdit[]) => dispatch(stretchClips(edits)),
    [dispatch],
  )

  return (
    <div 
      ref={ref}
      // the canvas draws dragged clips itself
      {...{ [OWN_DRAG_PREVIEW]: true }}
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
          loopEnabled={loopEnabled}
          onSeek={handleSeek}
          onClipsTrim={handleClipsTrim}
          onClipsStretch={handleClipsStretch}
        />
      </div>}
      <ClipMenuHost opener={openMenu} />
    </div>
  )
}

// memo: it takes no props, so pane focus and tab changes above it don't
// re-render it
export default React.memo(PlaylistCanvas)
