import React from 'react'
import { useStore } from 'react-redux'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import { PatternRun, flushNoteSends, setPatternNotes } from '../../state/slices/sequencerSlice'
import {
  selectPianoRollSnapEnabled, selectPianoRollSnapNote, selectPianoRollTool,
} from '../../state/slices/pianoRollSlice'
import { selectFocusedView } from '../../state/slices/windowSlice'
import { ClipSource, sameCommand } from '../../state/clipCommands'
import {
  NoteCommand, NoteShortcut, noteCommand, setNoteCommandRunner,
} from '../../state/noteCommands'
import { RAZOR_CURSOR } from '../../editing/cursors'
import { Edge, edgeAt } from '../../editing/edges'
import {
  Lasso, Point, ShiftPress, itemAt, lassoFrom, lassoRect, lassoSelection, shiftRelease, startsLasso,
} from '../../editing/lasso'
import { DRAG_THRESHOLD_PX, wheelIntent } from '../../editing/view'
import { useThemeChange } from '../../theme/canvas'
import { Note, Pattern, STEPS_PER_BEAT, synthNotes } from '../../types/sequencer'
import { View } from '../../types/ui'
import { isDeleteKey, isTextTarget, isTypingTarget } from '../../util/keys'
import { editShortcutHandler } from '../../util/shortcuts'
import { createAuditioner } from './audition'
import {
  DEFAULT_STEP_WIDTH, GridScene, KEYS_WIDTH, ROW_HEIGHT, RULER_HEIGHT,
  contentHeight, contentWidth, drawGrid, drawKeys, drawRuler, fitCanvas, pitchAtY, rowAtY, rowTop, stepAtX, zoomGrid,
} from './draw'
import {
  DEFAULT_LENGTH, centrePitch, clampPitch, cutNotes, drawnLength, fitSelection, moveNotes, newNote, noteBoxes,
  noteCut, pasteStep, pressKind, rollSnapping, sameNotes, snapStep, trimNotes,
} from './notes'
import { clamp } from '../../util/helpers'

// a place on the grid: CSS pixels from the whole grid's top-left, the step
// (fractional), row (fractional, down from the highest pitch) and pitch
// there, and where the mouse is on the page
interface GridPoint {
  x: number
  y: number
  step: number
  row: number
  pitch: number
  screen: Point
  // over the grid in view (not its scroll bars, nor outside it)
  inside: boolean
}

// what a press on the grid is doing until the mouse is let go
type Press =
  // shift held: a click (toggling the note under it) or, once dragged, a box
  | { kind: 'shift', press: ShiftPress }
  | { kind: 'lasso', lasso: Lasso }
  // on empty space: a click clears the selection, a drag pans
  | { kind: 'empty', from: Point, last: Point, panning: boolean }
  // the Cut tool on a note: cuts it if let go where it went down
  | { kind: 'cut', from: Point, index: number }
  // drawing a new note (at `index`): dragging right sets its length
  | { kind: 'draw', origin: Note[], last: Note[], index: number, from: Point, drawn: boolean }
  // moving the selection by the note grabbed (`index`), the mouse `grab`
  // steps into it
  | {
    kind: 'move', origin: Note[], last: Note[], index: number, indices: number[],
    grab: number, from: Point, active: boolean
  }
  // trimming the selection by an edge of the note grabbed, the mouse `grab`
  // steps before the edge
  | {
    kind: 'edge', origin: Note[], last: Note[], index: number, indices: number[],
    edge: Edge, grab: number, from: Point, active: boolean
  }

type NoteDrag = Extract<Press, { origin: Note[] }>
const isNoteDrag = (press: Press | null): press is NoteDrag =>
  !!press && (press.kind === 'draw' || press.kind === 'move' || press.kind === 'edge')

// what's drawn over the notes: a selection box, the Cut tool's line, a bar
// on an edge
type Overlay = Pick<GridScene, 'lasso' | 'cut' | 'edgeBar'>
const NO_OVERLAY: Overlay = { lasso: null, cut: null, edgeBar: null }
const sameOverlay = (a: Overlay, b: Overlay) => a === b || JSON.stringify(a) === JSON.stringify(b)

const NO_SELECTION: ReadonlySet<number> = new Set()

// how far the mouse moves before a press on empty grid draws by length
const DRAW_THRESHOLD_PX = 3

// WebKit (the macOS webview) reports trackpad pinches as gesture events,
// not ⌘+wheel. not in the DOM typings
interface GestureEvent extends UIEvent {
  scale: number
  clientX?: number
}

const distance = (a: Point, b: Point) => Math.hypot(b.x - a.x, b.y - a.y)

interface Props {
  pattern: Pattern
  // the synth whose notes are edited, which auditions pitches
  synthId: number
  // the other synths' notes, shown faintly behind
  ghosts: Note[]
  stepsPerBar: number
  // the pattern playing on its own, if any (for the playhead)
  run: PatternRun | null
}

// a synth's notes in a pattern against a piano keyboard, edited with the
// same tools as the playlist's clips (see src/editing/): draw, select, move,
// trim, cut, copy and paste them. every change replaces that synth's notes
// in the engine, a drag being one step to undo
const NoteGrid = ({ pattern, synthId, ghosts, stepsPerBar, run }: Props) => {
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const themeChange = useThemeChange()
  const tool = useAppSelector(selectPianoRollTool)
  const snapEnabled = useAppSelector(selectPianoRollSnapEnabled)
  const snapNote = useAppSelector(selectPianoRollSnapNote)
  const notes = synthNotes(pattern, synthId)
  const { length } = pattern

  // the notes as they're being dragged, shown ahead of the engine's answers
  const [draft, setDraft] = React.useState<Note[] | null>(null)
  const [selection, setSelection] = React.useState<ReadonlySet<number>>(NO_SELECTION)
  const [overlay, setOverlayState] = React.useState<Overlay>(NO_OVERLAY)
  const [pressedKey, setPressedKey] = React.useState<number | null>(null)
  const [size, setSize] = React.useState({ width: 0, height: 0 })
  const [scroll, setScroll] = React.useState({ x: 0, y: 0 })
  const [stepWidth, setStepWidth] = React.useState(DEFAULT_STEP_WIDTH)

  const shown = draft ?? notes
  const selected = fitSelection(selection, shown)

  const containerRef = React.useRef<HTMLDivElement>(null)
  const scrollerRef = React.useRef<HTMLDivElement>(null)
  const gridRef = React.useRef<HTMLCanvasElement>(null)
  const rulerRef = React.useRef<HTMLCanvasElement>(null)
  const keysRef = React.useRef<HTMLCanvasElement>(null)
  const playheadRef = React.useRef<HTMLDivElement>(null)
  const press = React.useRef<Press | null>(null)
  // a plain click draws a note as long as the last one drawn or trimmed
  const lastLength = React.useRef(DEFAULT_LENGTH)
  // the mouse, last seen: where on the page, and with which keys held
  const mouse = React.useRef<{ client: Point, shift: boolean } | null>(null)
  // Option held: nothing snaps
  const altHeld = React.useRef(false)
  // the step width and scroll a zoom asked for, ahead of the grid being
  // laid out again at that width
  const stepWidthNow = React.useRef(stepWidth)
  const pendingScroll = React.useRef<number | null>(null)
  // the scroll the last zoom asked for, to the fraction of a pixel
  const exactScroll = React.useRef<number | null>(null)
  const audition = React.useMemo(createAuditioner, [])

  // the latest of these, for listeners on the window. the notes and the
  // selection are brought up to date as soon as a command changes them, so
  // commands that arrive faster than the page draws act on each other's
  // results (⌘R pressed three times lays three copies end to end)
  const latest = React.useRef({ notes, selected, patternId: pattern.id, synthId, length })
  latest.current = { notes, selected, patternId: pattern.id, synthId, length }

  const setOverlay = (next: Overlay) => setOverlayState(prev => (sameOverlay(prev, next) ? prev : next))

  const select = (next: ReadonlySet<number>) => {
    latest.current.selected = next
    setSelection(next)
  }

  // nothing selected in another pattern or synth, and nothing left sounding
  React.useEffect(() => {
    setSelection(NO_SELECTION)
    setDraft(null)
    setOverlayState(NO_OVERLAY)
    press.current = null
  }, [pattern.id, synthId])
  React.useEffect(() => () => audition.stop(), [audition])

  // another tool: what the last one showed under the mouse goes
  React.useEffect(() => {
    setOverlayState(prev => (press.current ? prev : NO_OVERLAY))
  }, [tool])

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

  // a pattern (or another synth's notes) opens with its notes in the
  // middle, or C3 to C5 when it has none
  const centred = React.useRef<string | null>(null)
  React.useEffect(() => {
    const scroller = scrollerRef.current
    const showing = `${pattern.id}:${synthId}`
    if(!scroller || !size.height || centred.current === showing) return
    centred.current = showing
    scroller.scrollTop = Math.max(0, rowTop(centrePitch(notes)) + ROW_HEIGHT / 2 - size.height / 2)
    scroller.scrollLeft = 0
    setScroll({ x: scroller.scrollLeft, y: scroller.scrollTop })
  }, [pattern.id, synthId, size.height, notes])

  // zoomed: the grid is as wide as it's now been laid out, so the scroll
  // the zoom asked for can be made
  React.useLayoutEffect(() => {
    const scroller = scrollerRef.current
    if(!scroller || pendingScroll.current === null) return
    scroller.scrollLeft = pendingScroll.current
    pendingScroll.current = null
    setScroll({ x: scroller.scrollLeft, y: scroller.scrollTop })
  }, [stepWidth])

  // drawn again whenever something shown changes (not every frame)
  React.useEffect(() => {
    const view = { width: size.width, height: size.height, scrollX: scroll.x, scrollY: scroll.y, stepWidth }
    const grid = fitCanvas(gridRef.current, size.width, size.height)
    if(grid) drawGrid(grid, view, { length, stepsPerBar, notes: shown, ghosts, selected, ...overlay })
    const ruler = fitCanvas(rulerRef.current, size.width, RULER_HEIGHT)
    if(ruler) drawRuler(ruler, view, length, stepsPerBar)
  }, [size, scroll, stepWidth, length, stepsPerBar, shown, ghosts, selected, overlay, themeChange])

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
      line.style.transform = `translateX(${Math.round(step * stepWidthNow.current - scrollX.current)}px)`
      frame = requestAnimationFrame(follow)
    }
    follow()
    return () => cancelAnimationFrame(frame)
  }, [run, pattern.id, length, stepWidth])

  // what a drag, a cut or something drawn snaps to just now
  const snapping = (bypass = altHeld.current) => rollSnapping(snapEnabled, snapNote, bypass)

  // a change to the notes: shown now, and sent (at most every so often
  // while dragging; see setPatternNotes)
  const change = (next: Note[]) => {
    setDraft(next)
    dispatch(setPatternNotes(pattern.id, synthId, next))
  }

  // a change that's finished: sent now
  const commit = (next: Note[]) => {
    latest.current.notes = next
    dispatch(setPatternNotes(pattern.id, synthId, next))
    flushNoteSends()
  }

  const sound = (pitch: number) => {
    audition.play(synthId, pitch, true)
  }

  // where the mouse is on the grid
  const pointAt = (client: Point): GridPoint => {
    const scroller = scrollerRef.current
    const rect = scroller?.getBoundingClientRect()
    const left = client.x - (rect?.left ?? 0)
    const top = client.y - (rect?.top ?? 0)
    const x = left + (scroller?.scrollLeft ?? 0)
    const y = top + (scroller?.scrollTop ?? 0)
    return {
      x,
      y,
      step: stepAtX(x, stepWidthNow.current),
      row: rowAtY(y),
      pitch: clampPitch(pitchAtY(y)),
      screen: client,
      inside: !!scroller && left >= 0 && top >= 0 && left < scroller.clientWidth && top < scroller.clientHeight,
    }
  }

  const clientOf = (ev: { clientX: number, clientY: number }): Point => ({ x: ev.clientX, y: ev.clientY })

  // the note under a point (the one drawn last, on top), and the edge of it
  // a press there would grab, if any
  const noteAt = (list: Note[], point: GridPoint): number | null =>
    itemAt({ x: point.step, y: point.row }, noteBoxes(list))
  const edgeOf = (note: Note, point: GridPoint): Edge | null =>
    edgeAt(point.x - note.start * stepWidthNow.current, note.length * stepWidthNow.current)

  const setCursor = (cursor: string) => {
    const scroller = scrollerRef.current
    if(scroller && scroller.style.cursor !== cursor) scroller.style.cursor = cursor
  }

  // with nothing pressed: what a press here would do, shown by the cursor,
  // the Cut tool's line and the bar on an edge
  const hover = (point: GridPoint, shift: boolean) => {
    if(!point.inside) {
      setOverlay(NO_OVERLAY)
      setCursor('default')
      return
    }
    const hit = noteAt(notes, point)

    if(tool === 'cut') {
      // (with shift held a click selects, rather than cuts)
      const cut = hit === null || shift ? null : noteCut(notes, hit, selected, point.step, snapping())
      setOverlay({ ...NO_OVERLAY, cut })
      setCursor(RAZOR_CURSOR)
      return
    }

    const edge = hit === null || shift ? null : edgeOf(notes[hit], point)
    setOverlay({ ...NO_OVERLAY, edgeBar: edge && hit !== null ? { ids: [hit], edge } : null })
    if(edge) setCursor('ew-resize')
    else setCursor(hit === null ? 'default' : 'grab')
  }

  // a press, as the mouse moves (or Option goes down or up, or the grid
  // scrolls under it)
  const follow = (point: GridPoint) => {
    const d = press.current
    if(!d) return

    if(d.kind === 'shift') {
      // far enough from where it went down, it's a box
      if(!startsLasso(d.press, point.screen, DRAG_THRESHOLD_PX)) return
      press.current = { kind: 'lasso', lasso: lassoFrom(d.press, selected) }
      follow(point)
      return
    }

    if(d.kind === 'lasso') {
      d.lasso.end = { x: point.step, y: point.row }
      select(lassoSelection(d.lasso, noteBoxes(notes)))
      setOverlay({ ...NO_OVERLAY, lasso: lassoRect(d.lasso) })
      setCursor('crosshair')
      return
    }

    if(d.kind === 'empty') {
      if(!d.panning && distance(d.from, point.screen) <= DRAG_THRESHOLD_PX) return
      d.panning = true
      const scroller = scrollerRef.current
      if(scroller) {
        scroller.scrollLeft -= point.screen.x - d.last.x
        scroller.scrollTop -= point.screen.y - d.last.y
      }
      d.last = point.screen
      return
    }

    if(d.kind === 'cut') return

    let next = d.last
    if(d.kind === 'draw') {
      // up or down onto another key: the note goes there, and is heard.
      // dragging right (past a few pixels) sets its length
      const note = d.last[d.index]
      if(!d.drawn && Math.abs(point.screen.x - d.from.x) >= DRAW_THRESHOLD_PX) d.drawn = true
      const drawnTo = d.drawn ? drawnLength(note.start, point.step, length, snapping()) : note.length
      if(point.pitch === note.pitch && drawnTo === note.length) return
      next = [...d.origin, { ...note, pitch: point.pitch, length: drawnTo }]
      if(point.pitch !== note.pitch) sound(point.pitch)
    } else {
      if(!d.active && distance(d.from, point.screen) <= DRAG_THRESHOLD_PX) return
      d.active = true

      if(d.kind === 'move') {
        setCursor('grabbing')
        const start = snapStep(point.step - d.grab, snapping())
        next = moveNotes(d.origin, d.indices, d.index, start, point.pitch, length)
        // heard again on its new pitch
        if(next[d.index].pitch !== d.last[d.index].pitch) sound(next[d.index].pitch)
      } else {
        setCursor('ew-resize')
        setOverlay({ ...NO_OVERLAY, edgeBar: { ids: d.indices, edge: d.edge } })
        const target = snapStep(point.step + d.grab, snapping())
        next = trimNotes(d.origin, d.indices, d.index, d.edge, target, length)
      }
    }

    if(sameNotes(next, d.last)) return
    d.last = next
    change(next)
  }

  // what's under the mouse, or what it's dragging, again: after Option or
  // the grid's scroll changed under a still mouse
  const refresh = () => {
    const at = mouse.current
    if(!at) return
    const point = pointAt(at.client)
    if(press.current) follow(point)
    else hover(point, at.shift)
  }

  // a press is let go, at `point`
  const endPress = (point: GridPoint, cancelled: boolean) => {
    const d = press.current
    press.current = null
    if(!d) return
    audition.stop()
    setOverlay(NO_OVERLAY)

    if(d.kind === 'shift') {
      if(cancelled) return
      select(shiftRelease(
        d.press,
        { screen: point.screen, point: { x: point.step, y: point.row } },
        DRAG_THRESHOLD_PX,
        selected,
        noteBoxes(notes),
      ))
      return
    }
    if(d.kind === 'lasso') {
      if(cancelled) return
      d.lasso.end = { x: point.step, y: point.row }
      select(lassoSelection(d.lasso, noteBoxes(notes)))
      return
    }
    if(d.kind === 'empty') {
      // a click (not a pan) clears the selection
      if(!d.panning && !cancelled) select(NO_SELECTION)
      return
    }
    if(d.kind === 'cut') {
      // a click on the note pressed cuts it. dragged away, it's nothing
      if(cancelled || distance(d.from, point.screen) > DRAG_THRESHOLD_PX) return
      if(noteAt(notes, point) !== d.index) return
      const cut = noteCut(notes, d.index, selected, point.step, snapping())
      if(!cut) return
      const result = cutNotes(notes, cut.ids, cut.step)
      // a selected note cut is still selected: both halves of it
      const halves = [...result.halves].filter(([i]) => selected.has(i)).map(([, half]) => half)
      if(halves.length) select(new Set([...selected, ...halves]))
      commit(result.notes)
      return
    }

    if(d.kind === 'draw' && d.drawn) lastLength.current = d.last[d.index].length
    if(d.kind === 'edge' && d.active) lastLength.current = d.last[d.index].length

    // the last change of a drag may still be waiting to be sent: now
    if(!sameNotes(d.last, d.origin)) flushNoteSends()
    setDraft(null)
  }

  // Escape, or the window losing the mouse, mid-press: notes being drawn,
  // moved or trimmed go back as they were, and a box or a click is dropped.
  // `clear`: and the selection is cleared too, unless notes went back
  const dropPress = (clear: boolean) => {
    const d = press.current
    if(!d) return false
    press.current = null
    audition.stop()
    setOverlay(NO_OVERLAY)

    if(isNoteDrag(d)) {
      if(!sameNotes(d.last, d.origin)) commit(d.origin)
      setDraft(null)
    } else if(clear) {
      select(NO_SELECTION)
    }
    return true
  }

  // the latest of these, for the listeners on the window
  const live = React.useRef({ refresh, dropPress, pointAt, snapping })
  live.current = { refresh, dropPress, pointAt, snapping }

  // a press was let go: what's under the mouse is shown again once the
  // notes it changed are drawn (a cut line on the half just cut, not on the
  // whole note it was)
  const rehover = React.useRef(false)
  React.useEffect(() => {
    if(!rehover.current || press.current) return
    rehover.current = false
    refresh()
  })

  // the commands: from the keys (Delete, Escape and the ⌘ shortcuts), the
  // system's copy, cut and paste events, and the Edit menu. while the Piano
  // Roll is the focused pane's tab, nothing that takes typing has the key,
  // and nothing's being dragged. a shortcut can arrive by more than one
  // route, and is acted on once
  React.useEffect(() => {
    const focused = () => selectFocusedView(store.getState()) === View.PianoRoll
    let last: { command: NoteShortcut, source: ClipSource, at: number } | null = null

    const apply = (command: NoteCommand) => {
      // not in the middle of a drag, whichever way the command came
      if(press.current) return
      const { notes: current, selected: chosen, patternId, synthId: synth, length: patternLength } = latest.current

      // a paste goes where the mouse is over the grid, or else at the
      // first snap line in view
      const scroller = scrollerRef.current
      const at = mouse.current ? live.current.pointAt(mouse.current.client) : null
      const pasteAt = pasteStep(
        at?.inside ? at.step : null,
        (scroller?.scrollLeft ?? 0) / stepWidthNow.current,
        live.current.snapping(false),
      )

      const outcome = noteCommand(command, {
        notes: current, selected: chosen, patternLength, pasteAt, focused: focused(),
      })
      if(!outcome) return
      if(outcome.notes) {
        latest.current.notes = outcome.notes
        dispatch(setPatternNotes(patternId, synth, outcome.notes))
        flushNoteSends()
      }
      if(outcome.selected) {
        latest.current.selected = outcome.selected
        setSelection(outcome.selected)
      }
    }

    const run = (command: NoteShortcut, source: ClipSource) => {
      const now = Date.now()
      if(sameCommand(last, command, source, now)) return
      last = { command, source, at: now }
      apply(command)
    }

    // (undo and redo are handled in useEditCommands)
    const onShortcut = editShortcutHandler(command => {
      if(command === 'undo' || command === 'redo') return
      run(command, 'key')
    })

    const onKeyDown = (ev: KeyboardEvent) => {
      if(!focused() || isTypingTarget(ev.target)) return

      if(ev.key === 'Escape') {
        // a press: the notes go back, or a box is dropped
        if(live.current.dropPress(true)) return
        apply('escape')
        return
      }
      if(ev.repeat || press.current) return

      if(isDeleteKey(ev)) {
        if(!latest.current.selected.size) return
        // Backspace would otherwise navigate back in some webviews
        ev.preventDefault()
        apply('delete')
        return
      }
      onShortcut(ev)
    }

    // the system's own events, which in a text field act on the text. here
    // they're taken over, so the system clipboard is left alone
    const onClipboard = (ev: ClipboardEvent) => {
      if(isTextTarget(ev.target) || isTextTarget(document.activeElement)) return
      if(!focused()) return
      ev.preventDefault()
      if(ev.type === 'copy' || ev.type === 'cut' || ev.type === 'paste') run(ev.type, 'clipboard-event')
    }

    setNoteCommandRunner(run)
    window.addEventListener('keydown', onKeyDown)
    document.addEventListener('copy', onClipboard)
    document.addEventListener('cut', onClipboard)
    document.addEventListener('paste', onClipboard)
    return () => {
      setNoteCommandRunner(null)
      window.removeEventListener('keydown', onKeyDown)
      document.removeEventListener('copy', onClipboard)
      document.removeEventListener('cut', onClipboard)
      document.removeEventListener('paste', onClipboard)
    }
  }, [store, dispatch])

  // Option, held or let go, changes what snaps. the window losing the mouse
  // mid-press may never send the release: the press is dropped
  React.useEffect(() => {
    const onAlt = (ev: KeyboardEvent) => {
      if(altHeld.current === ev.altKey) return
      altHeld.current = ev.altKey
      live.current.refresh()
    }
    const onBlur = () => {
      altHeld.current = false
      live.current.dropPress(false)
    }
    window.addEventListener('keydown', onAlt)
    window.addEventListener('keyup', onAlt)
    window.addEventListener('blur', onBlur)
    return () => {
      window.removeEventListener('keydown', onAlt)
      window.removeEventListener('keyup', onAlt)
      window.removeEventListener('blur', onBlur)
    }
  }, [])

  // zoom along time by `factor` about the mouse at `clientX`. quick zooms
  // build on each other before the grid is laid out again
  const zoomBy = React.useCallback((factor: number, clientX: number) => {
    const scroller = scrollerRef.current
    if(!scroller) return
    const rect = scroller.getBoundingClientRect()
    const aroundX = clamp(clientX - rect.left, 0, scroller.clientWidth)
    // (the scroller keeps whole pixels. zooming on from what it kept, the
    // step under the mouse would creep a fraction of a pixel each time)
    const kept = scroller.scrollLeft
    const asked = exactScroll.current
    const from = pendingScroll.current ?? (asked !== null && Math.abs(asked - kept) < 1 ? asked : kept)
    const next = zoomGrid(stepWidthNow.current, from, factor, aroundX)
    if(next.stepWidth === stepWidthNow.current) return

    stepWidthNow.current = next.stepWidth
    pendingScroll.current = next.scrollLeft
    exactScroll.current = next.scrollLeft
    setStepWidth(next.stepWidth)
  }, [])

  // the wheel or a two-finger swipe scrolls: up and down through the
  // pitches, left and right (or shift+wheel) along time. ⌘+wheel, and a
  // pinch, zoom along time about the mouse. anywhere over the grid, the
  // ruler or the keyboard
  React.useEffect(() => {
    const container = containerRef.current
    if(!container) return undefined

    const onWheel = (ev: WheelEvent) => {
      const scroller = scrollerRef.current
      if(!scroller) return
      ev.preventDefault()
      const wheel = wheelIntent(ev, scroller.clientHeight)
      if(wheel.kind === 'zoom') {
        zoomBy(wheel.factor, ev.clientX)
        return
      }
      scroller.scrollLeft += wheel.dx
      scroller.scrollTop += wheel.dy
    }

    // (preventing the default also stops a pinch zooming the whole window)
    let pinchScale = 1
    const onGestureStart = (ev: Event) => {
      ev.preventDefault()
      pinchScale = 1
    }
    const onGestureChange = (ev: Event) => {
      ev.preventDefault()
      const { scale, clientX } = ev as GestureEvent
      const x = clientX ?? mouse.current?.client.x ?? 0
      zoomBy(scale / pinchScale, x)
      pinchScale = scale
    }

    container.addEventListener('wheel', onWheel, { passive: false })
    container.addEventListener('gesturestart', onGestureStart)
    container.addEventListener('gesturechange', onGestureChange)
    return () => {
      container.removeEventListener('wheel', onWheel)
      container.removeEventListener('gesturestart', onGestureStart)
      container.removeEventListener('gesturechange', onGestureChange)
    }
  }, [zoomBy])

  const onPointerDown = (ev: React.PointerEvent<HTMLDivElement>) => {
    altHeld.current = ev.altKey
    if(ev.button !== 0 || press.current) return
    const point = pointAt(clientOf(ev))
    if(!point.inside) return
    ev.currentTarget.focus()

    const hit = noteAt(notes, point)
    const edge = hit === null ? null : edgeOf(notes[hit], point)
    const kind = pressKind({ tool, shift: ev.shiftKey, onNote: hit !== null, onEdge: !!edge, inPattern: point.step < length })

    ev.currentTarget.setPointerCapture?.(ev.pointerId)
    setOverlay(NO_OVERLAY)

    if(kind === 'shift') {
      press.current = { kind, press: { screen: point.screen, point: { x: point.step, y: point.row }, itemId: hit } }
      return
    }
    if(kind === 'empty') {
      press.current = { kind, from: point.screen, last: point.screen, panning: false }
      return
    }
    if(kind === 'cut' && hit !== null) {
      press.current = { kind, from: point.screen, index: hit }
      return
    }

    if(hit !== null) {
      // a note that isn't selected is selected on its own. a selected one
      // brings the rest of the selection with it
      const indices = selected.has(hit) ? [...selected] : [hit]
      if(!selected.has(hit)) select(new Set([hit]))
      const note = notes[hit]

      if(kind === 'edge' && edge) {
        const at = edge === 'left' ? note.start : note.start + note.length
        press.current = {
          kind, origin: notes, last: notes, index: hit, indices, edge, grab: at - point.step, from: point.screen, active: false,
        }
        setOverlay({ ...NO_OVERLAY, edgeBar: { ids: indices, edge } })
        return
      }

      press.current = {
        kind: 'move', origin: notes, last: notes, index: hit, indices, grab: point.step - note.start, from: point.screen, active: false,
      }
      sound(note.pitch)
      return
    }

    // drawing: a new note where the mouse is, as long as the last one
    const next = [...notes, newNote(point.step, point.pitch, lastLength.current, length, snapping())]
    const index = next.length - 1
    select(new Set([index]))
    press.current = { kind: 'draw', origin: notes, last: next, index, from: point.screen, drawn: false }
    change(next)
    sound(next[index].pitch)
  }

  const onPointerMove = (ev: React.PointerEvent<HTMLDivElement>) => {
    altHeld.current = ev.altKey
    mouse.current = { client: clientOf(ev), shift: ev.shiftKey }
    const point = pointAt(mouse.current.client)
    if(press.current) follow(point)
    else hover(point, ev.shiftKey)
  }

  const onPointerUp = (ev: React.PointerEvent<HTMLDivElement>) => {
    // another button let go isn't the end of the press
    if(ev.button !== 0) return
    altHeld.current = ev.altKey
    mouse.current = { client: clientOf(ev), shift: ev.shiftKey }
    const point = pointAt(mouse.current.client)
    endPress(point, false)
    hover(point, ev.shiftKey)
    rehover.current = true
  }

  const onPointerCancel = (ev: React.PointerEvent<HTMLDivElement>) => {
    endPress(pointAt(clientOf(ev)), true)
  }

  const onPointerLeave = () => {
    if(press.current) return
    mouse.current = null
    setOverlay(NO_OVERLAY)
  }

  // right-click a note to delete it
  const onContextMenu = (ev: React.MouseEvent<HTMLDivElement>) => {
    ev.preventDefault()
    if(press.current) return
    const point = pointAt(clientOf(ev))
    const hit = point.inside ? noteAt(notes, point) : null
    if(hit === null) return
    select(NO_SELECTION)
    setOverlay(NO_OVERLAY)
    commit(notes.filter((_, i) => i !== hit))
  }

  // the keyboard: a key sounds while it's held
  const keyAt = (ev: { clientY: number }) => {
    const rect = keysRef.current?.getBoundingClientRect()
    return clampPitch(pitchAtY(ev.clientY - (rect?.top ?? 0) + scroll.y))
  }
  const pressKey = (pitch: number) => {
    setPressedKey(pitch)
    audition.play(synthId, pitch, false)
  }
  const releaseKey = () => {
    if(pressedKey === null) return
    setPressedKey(null)
    audition.stop()
  }

  return (
    <div ref={containerRef} className='relative flex-1 min-h-0 overflow-hidden rounded-md'>
      <div
        className='absolute top-0 left-0 bg-panel border-b border-r border-line-strong'
        style={{ width: KEYS_WIDTH, height: RULER_HEIGHT }}
      />
      <canvas
        ref={rulerRef}
        className='absolute top-0'
        style={{ left: KEYS_WIDTH, width: size.width, height: RULER_HEIGHT }}
      />
      <canvas
        ref={keysRef}
        className='absolute left-0 cursor-pointer touch-none'
        style={{ top: RULER_HEIGHT, width: KEYS_WIDTH, height: size.height }}
        title='Click a key to hear it'
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
        // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- it takes Delete, Escape and the ⌘ shortcuts
        tabIndex={0}
        className='absolute overflow-auto outline-none touch-none'
        style={{ left: KEYS_WIDTH, top: RULER_HEIGHT, right: 0, bottom: 0 }}
        data-testid='note-grid'
        onScroll={ev => {
          setScroll({ x: ev.currentTarget.scrollLeft, y: ev.currentTarget.scrollTop })
          // a drag follows the grid scrolling under the mouse
          if(press.current) refresh()
        }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerCancel}
        onPointerLeave={onPointerLeave}
        onContextMenu={onContextMenu}
      >
        <div style={{ width: contentWidth(length, stepWidth), height: contentHeight() }} />
      </div>
    </div>
  )
}

export default NoteGrid
