import React from 'react'
import { useStore } from 'react-redux'
import { useAppDispatch } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import { PatternRun, flushNoteSends, setPatternNotes } from '../../state/slices/sequencerSlice'
import { selectFocusedView } from '../../state/slices/windowSlice'
import { setNoteSelectAllRunner } from '../../state/noteCommands'
import { useThemeChange } from '../../theme/canvas'
import { Note, Pattern, STEPS_PER_BEAT, notesOf } from '../../types/sequencer'
import { View } from '../../types/ui'
import { isDeleteKey, isTypingTarget } from '../../util/keys'
import { editShortcut } from '../../util/shortcuts'
import { createAuditioner } from './audition'
import {
  EDGE_PX, GridScene, KEYS_WIDTH, ROW_HEIGHT, RULER_HEIGHT, STEP_WIDTH,
  contentHeight, contentWidth, drawGrid, drawKeys, drawRuler, fitCanvas, pitchAtY, rowTop, stepAtX,
} from './draw'
import {
  DEFAULT_LENGTH, allIndices, centrePitch, clampPitch, deleteNotes, drawnLength, fitSelection, hitNote,
  moveNotes, newNote, notesInBox, resizeNotes, sameNotes, toggleSelected,
} from './notes'

// a place on the grid: CSS pixels from the whole grid's top-left, and the
// step (fractional) and pitch there
interface Point {
  x: number
  y: number
  step: number
  pitch: number
}

// what a press on the grid is doing until the mouse is let go
type Drag =
  // drawing a new note (at `index`): dragging right sets its length
  | { kind: 'draw', origin: Note[], from: Point, index: number, last: Note[], drawn: boolean }
  // moving the selection by the note grabbed (`index`), or resizing it
  | { kind: 'move' | 'resize', origin: Note[], from: Point, index: number, indices: number[], last: Note[] }
  // a rubber band, adding what it touches to what was selected
  | { kind: 'box', from: Point, base: number[] }

// how far the mouse moves before a press on empty grid draws by length
const DRAW_THRESHOLD_PX = 3

interface Props {
  pattern: Pattern
  stepsPerBar: number
  // the track whose instrument auditions pitches, or null
  track: number | null
  // the pattern playing on its own, if any (for the playhead)
  run: PatternRun | null
}

// a pattern's notes against a piano keyboard: draw, move, resize, select
// and delete them. every change replaces the pattern's notes in the engine,
// a drag being one step to undo
const NoteGrid = ({ pattern, stepsPerBar, track, run }: Props) => {
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const themeChange = useThemeChange()
  const notes = notesOf(pattern)
  const { length } = pattern

  // the notes as they're being dragged, shown ahead of the engine's answers
  const [draft, setDraft] = React.useState<Note[] | null>(null)
  const [selection, setSelection] = React.useState<number[]>([])
  const [box, setBox] = React.useState<GridScene['box']>(null)
  const [pressedKey, setPressedKey] = React.useState<number | null>(null)
  const [size, setSize] = React.useState({ width: 0, height: 0 })
  const [scroll, setScroll] = React.useState({ x: 0, y: 0 })

  const shown = draft ?? notes
  const selected = fitSelection(selection, shown)
  const selectedSet = React.useMemo(() => new Set(selected), [selected])

  const scrollerRef = React.useRef<HTMLDivElement>(null)
  const gridRef = React.useRef<HTMLCanvasElement>(null)
  const rulerRef = React.useRef<HTMLCanvasElement>(null)
  const keysRef = React.useRef<HTMLCanvasElement>(null)
  const playheadRef = React.useRef<HTMLDivElement>(null)
  const drag = React.useRef<Drag | null>(null)
  // a plain click draws a note as long as the last one drawn or resized
  const lastLength = React.useRef(DEFAULT_LENGTH)
  const audition = React.useMemo(createAuditioner, [])
  // the latest of these, for listeners on the window
  const latest = React.useRef({ notes, selected, patternId: pattern.id })
  latest.current = { notes, selected, patternId: pattern.id }

  // nothing selected in another pattern, and nothing left sounding
  React.useEffect(() => {
    setSelection([])
    setDraft(null)
    drag.current = null
  }, [pattern.id])
  React.useEffect(() => () => audition.stop(), [audition])

  // follow the grid's size: window resizes, split pane dividers
  React.useEffect(() => {
    const scroller = scrollerRef.current
    if(!scroller) return undefined

    const measure = () => {
      const { clientWidth: width, clientHeight: height } = scroller
      // hidden behind another tab, keep the last size
      if(width && height) setSize(prev => (prev.width === width && prev.height === height ? prev : { width, height }))
    }
    measure()
    if(typeof ResizeObserver === 'undefined') return undefined

    const observer = new ResizeObserver(measure)
    observer.observe(scroller)
    return () => observer.disconnect()
  }, [])

  // a pattern opens with its notes in the middle, or C3 to C5 when it has
  // none
  const centred = React.useRef<number | null>(null)
  React.useEffect(() => {
    const scroller = scrollerRef.current
    if(!scroller || !size.height || centred.current === pattern.id) return
    centred.current = pattern.id
    scroller.scrollTop = Math.max(0, rowTop(centrePitch(notes)) + ROW_HEIGHT / 2 - size.height / 2)
    scroller.scrollLeft = 0
    setScroll({ x: scroller.scrollLeft, y: scroller.scrollTop })
  }, [pattern.id, size.height, notes])

  // drawn again whenever something shown changes (not every frame)
  React.useEffect(() => {
    const view = { width: size.width, height: size.height, scrollX: scroll.x, scrollY: scroll.y }
    const grid = fitCanvas(gridRef.current, size.width, size.height)
    if(grid) drawGrid(grid, view, { length, stepsPerBar, notes: shown, selected: selectedSet, box })
    const ruler = fitCanvas(rulerRef.current, size.width, RULER_HEIGHT)
    if(ruler) drawRuler(ruler, view, length, stepsPerBar)
  }, [size, scroll, length, stepsPerBar, shown, selectedSet, box, themeChange])

  React.useEffect(() => {
    const keys = fitCanvas(keysRef.current, KEYS_WIDTH, size.height)
    if(keys) drawKeys(keys, { height: size.height, scrollY: scroll.y }, pressedKey)
  }, [size.height, scroll.y, pressedKey, themeChange])

  // the playhead, while this pattern plays on its own: moved every frame,
  // without drawing the grid again
  const scrollX = React.useRef(0)
  scrollX.current = scroll.x
  React.useEffect(() => {
    const line = playheadRef.current
    if(!line) return undefined
    if(!run || run.patternId !== pattern.id || length <= 0 || run.beatIntervalMs <= 0) {
      line.style.display = 'none'
      return undefined
    }

    let frame = 0
    const follow = () => {
      const stepMs = run.beatIntervalMs / STEPS_PER_BEAT
      const step = (Math.max(0, Date.now() - run.startTimeMs) / stepMs) % length
      line.style.display = ''
      line.style.transform = `translateX(${Math.round(step * STEP_WIDTH - scrollX.current)}px)`
      frame = requestAnimationFrame(follow)
    }
    follow()
    return () => cancelAnimationFrame(frame)
  }, [run, pattern.id, length])

  // a change to the notes: shown now, and sent (at most every so often
  // while dragging; see setPatternNotes)
  const change = (next: Note[]) => {
    setDraft(next)
    dispatch(setPatternNotes(pattern.id, next))
  }

  // a change that's finished: sent now
  const commit = (next: Note[]) => {
    dispatch(setPatternNotes(pattern.id, next))
    flushNoteSends()
  }

  const sound = (pitch: number) => {
    if(track !== null) audition.play(track, pitch, true)
  }

  // the keys: Delete removes the selection, Escape clears it, ⌘A selects
  // every note. while the Piano Roll is the focused pane's tab, and nothing
  // that takes typing has the key
  React.useEffect(() => {
    const focused = () => selectFocusedView(store.getState()) === View.PianoRoll

    const onKeyDown = (ev: KeyboardEvent) => {
      if(!focused() || isTypingTarget(ev.target) || drag.current) return
      const { notes: current, selected: chosen, patternId } = latest.current

      if(isDeleteKey(ev)) {
        if(!chosen.length || ev.repeat) return
        // Backspace would otherwise navigate back in some webviews
        ev.preventDefault()
        setSelection([])
        dispatch(setPatternNotes(patternId, deleteNotes(current, chosen)))
        flushNoteSends()
      } else if(ev.key === 'Escape') {
        setSelection([])
      } else if(editShortcut(ev) === 'select-all') {
        ev.preventDefault()
        setSelection(allIndices(current))
      }
    }

    // and Edit › Select All
    setNoteSelectAllRunner(() => {
      if(focused()) setSelection(allIndices(latest.current.notes))
    })
    window.addEventListener('keydown', onKeyDown)
    return () => {
      setNoteSelectAllRunner(null)
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [store, dispatch])

  // where the mouse is on the grid, and whether it's on it (not on a
  // scroll bar)
  const pointAt = (ev: { clientX: number, clientY: number }): Point & { inside: boolean } => {
    const scroller = scrollerRef.current
    const rect = scroller?.getBoundingClientRect()
    const left = ev.clientX - (rect?.left ?? 0)
    const top = ev.clientY - (rect?.top ?? 0)
    const x = left + (scroller?.scrollLeft ?? 0)
    const y = top + (scroller?.scrollTop ?? 0)
    return {
      x,
      y,
      step: stepAtX(x),
      pitch: clampPitch(pitchAtY(y)),
      inside: !!scroller && left >= 0 && top >= 0 && left < scroller.clientWidth && top < scroller.clientHeight,
    }
  }

  const edgeSteps = EDGE_PX / STEP_WIDTH

  const onPointerDown = (ev: React.PointerEvent<HTMLDivElement>) => {
    if(ev.button !== 0 || drag.current) return
    const point = pointAt(ev)
    if(!point.inside) return
    ev.currentTarget.focus()

    const hit = hitNote(notes, point.step, point.pitch, edgeSteps)
    if(hit && ev.shiftKey) {
      setSelection(toggleSelected(selected, hit.index))
      return
    }
    // past the pattern's end there's nothing to draw
    if(!hit && point.step >= length) {
      if(!ev.shiftKey) setSelection([])
      return
    }

    ev.currentTarget.setPointerCapture?.(ev.pointerId)

    if(hit) {
      // a selected note drags the whole selection
      const indices = selected.includes(hit.index) ? selected : [hit.index]
      setSelection(indices)
      drag.current = {
        kind: hit.part === 'end' ? 'resize' : 'move',
        origin: notes,
        from: point,
        index: hit.index,
        indices,
        last: notes,
      }
      if(hit.part === 'body') sound(notes[hit.index].pitch)
      return
    }

    if(ev.shiftKey) {
      drag.current = { kind: 'box', from: point, base: selected }
      setBox({ x0: point.x, y0: point.y, x1: point.x, y1: point.y })
      return
    }

    const next = [...notes, newNote(point.step, point.pitch, lastLength.current, length)]
    const index = next.length - 1
    setSelection([index])
    drag.current = { kind: 'draw', origin: notes, from: point, index, last: next, drawn: false }
    change(next)
    sound(next[index].pitch)
  }

  const onPointerMove = (ev: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current
    const point = pointAt(ev)

    if(!d) {
      // what a press here would do
      const hit = point.inside ? hitNote(notes, point.step, point.pitch, edgeSteps) : null
      let cursor = 'default'
      if(hit) cursor = hit.part === 'end' ? 'ew-resize' : 'grab'
      ev.currentTarget.style.cursor = cursor
      return
    }

    if(d.kind === 'box') {
      setBox({ x0: d.from.x, y0: d.from.y, x1: point.x, y1: point.y })
      const touched = notesInBox(notes, d.from, point)
      setSelection(Array.from(new Set([...d.base, ...touched])).sort((a, b) => a - b))
      return
    }

    let next = d.last
    if(d.kind === 'draw') {
      if(!d.drawn && Math.abs(point.x - d.from.x) < DRAW_THRESHOLD_PX) return
      d.drawn = true
      const note = d.last[d.index]
      next = [...d.origin, { ...note, length: drawnLength(note.start, point.step, length) }]
    } else if(d.kind === 'move') {
      ev.currentTarget.style.cursor = 'grabbing'
      const steps = Math.floor(point.step) - Math.floor(d.from.step)
      next = moveNotes(d.origin, d.indices, steps, point.pitch - d.from.pitch, length)
      // heard again on its new pitch
      if(next[d.index].pitch !== d.last[d.index].pitch) sound(next[d.index].pitch)
    } else {
      next = resizeNotes(d.origin, d.indices, Math.round(point.step - d.from.step), length)
    }

    if(sameNotes(next, d.last)) return
    d.last = next
    change(next)
  }

  const endDrag = () => {
    const d = drag.current
    drag.current = null
    if(!d) return
    audition.stop()

    if(d.kind === 'box') {
      setBox(null)
      return
    }
    if(d.kind === 'draw' && d.drawn) lastLength.current = d.last[d.index].length
    if(d.kind === 'resize') lastLength.current = d.last[d.index].length

    // the last change of a drag may still be waiting to be sent: now
    if(!sameNotes(d.last, d.origin)) flushNoteSends()
    setDraft(null)
  }

  // right-click a note to delete it
  const onContextMenu = (ev: React.MouseEvent<HTMLDivElement>) => {
    ev.preventDefault()
    if(drag.current) return
    const point = pointAt(ev)
    const hit = point.inside ? hitNote(notes, point.step, point.pitch, edgeSteps) : null
    if(!hit) return
    setSelection([])
    commit(deleteNotes(notes, [hit.index]))
  }

  // the keyboard: a key sounds while it's held
  const keyAt = (ev: { clientY: number }) => {
    const rect = keysRef.current?.getBoundingClientRect()
    return clampPitch(pitchAtY(ev.clientY - (rect?.top ?? 0) + scroll.y))
  }
  const pressKey = (pitch: number) => {
    setPressedKey(pitch)
    if(track !== null) audition.play(track, pitch, false)
  }
  const releaseKey = () => {
    if(pressedKey === null) return
    setPressedKey(null)
    audition.stop()
  }

  // the keyboard and ruler scroll the grid
  const onWheel = (ev: React.WheelEvent) => {
    const scroller = scrollerRef.current
    if(!scroller) return
    scroller.scrollTop += ev.deltaY
    scroller.scrollLeft += ev.deltaX
  }

  return (
    <div className='relative flex-1 min-h-0 overflow-hidden rounded-md'>
      <div
        className='absolute top-0 left-0 bg-panel border-b border-r border-line-strong'
        style={{ width: KEYS_WIDTH, height: RULER_HEIGHT }}
      />
      <canvas
        ref={rulerRef}
        className='absolute top-0'
        style={{ left: KEYS_WIDTH, width: size.width, height: RULER_HEIGHT }}
        onWheel={onWheel}
      />
      <canvas
        ref={keysRef}
        className='absolute left-0 cursor-pointer touch-none'
        style={{ top: RULER_HEIGHT, width: KEYS_WIDTH, height: size.height }}
        title={track === null ? 'Drag an instrument onto a track in the Mixer to hear the keys' : 'Click a key to hear it'}
        onPointerDown={ev => { if(ev.button === 0) pressKey(keyAt(ev)) }}
        onPointerMove={ev => {
          // sliding along the keys plays each in turn
          if(pressedKey === null || !(ev.buttons & 1)) return
          const pitch = keyAt(ev)
          if(pitch !== pressedKey) pressKey(pitch)
        }}
        onPointerUp={releaseKey}
        onPointerCancel={releaseKey}
        onPointerLeave={releaseKey}
        onWheel={onWheel}
      />
      <canvas
        ref={gridRef}
        className='absolute'
        style={{ left: KEYS_WIDTH, top: RULER_HEIGHT, width: size.width, height: size.height }}
      />
      {/* the playhead, over the ruler and the grid */}
      <div className='absolute top-0 bottom-0 right-0 overflow-hidden pointer-events-none' style={{ left: KEYS_WIDTH }}>
        <div ref={playheadRef} className='absolute top-0 bottom-0 left-0 w-px bg-accent' style={{ display: 'none' }} />
      </div>
      <div
        ref={scrollerRef}
        role='application'
        aria-label='Notes'
        aria-roledescription='piano roll'
        // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- it takes Delete, Escape and ⌘A
        tabIndex={0}
        className='absolute overflow-auto outline-none touch-none'
        style={{ left: KEYS_WIDTH, top: RULER_HEIGHT, right: 0, bottom: 0 }}
        data-testid='note-grid'
        onScroll={ev => setScroll({ x: ev.currentTarget.scrollLeft, y: ev.currentTarget.scrollTop })}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onContextMenu={onContextMenu}
      >
        <div style={{ width: contentWidth(length), height: contentHeight() }} />
      </div>
    </div>
  )
}

export default NoteGrid
