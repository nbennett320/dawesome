// dragging a sample from the sidebar into the playlist or the Pattern
// Sequencer, a pattern from its pattern strip into the playlist, an effect
// onto a mixer channel, an instrument into the Pattern Sequencer, or a
// pane's tab to another pane
//
// this uses plain mouse events instead of HTML drag and drop (react-dnd):
// during an HTML drag the playlist canvas couldn't draw the clip following
// the mouse, while mouse events keep p5's draw loop running as normal. and
// with Tauri's file drop handler on (dragDropEnabled, for files dragged in
// from Finder), the page never hears of an HTML drag at all

// the sidebar browsers that have folders
export type FolderScope = 'recordings' | 'clips' | 'patterns'

// a file dragged in from outside the app that can be played, and the folder
// in the clips it goes in ("" for a file dragged in by itself, "Pack/Kicks"
// for one in a folder Pack). see scan_dropped_files in
// engine/src/daw/file_drop.rs
export interface DroppedFile {
  path: string
  folder: string
}

// something dragged in that won't be brought in, and why
export interface SkippedFile {
  path: string
  reason: string
}

// what's being dragged. a folder only moves within its browser: it can't be
// placed in the playlist or the Pattern Sequencer. an effect (from the
// sidebar) and a slot (a row of the Effects Rack) go onto a mixer channel or
// the rack. an instrument (from the sidebar) goes into the Pattern
// Sequencer, or onto a track's mixer strip. files come from outside the app
// (Finder): they're copied into the clips, and placed in the playlist or
// the Pattern Sequencer
export type DragItem =
  | { kind: 'sample', path: string }
  | { kind: 'pattern', patternId: number, label: string }
  | { kind: 'folder', scope: FolderScope, key: string, label: string }
  | { kind: 'effect', pluginId: string, label: string }
  | { kind: 'instrument', pluginId: string, label: string }
  | { kind: 'slot', slotId: number, label: string }
  | { kind: 'files', files: DroppedFile[], skipped: SkippedFile[] }
  // a pane's tab, to another pane's tabs or onto a pane to split it (see
  // components/panes/dragTab.ts)
  | { kind: 'tab', tabKey: string, label: string }

// what can be dropped into the playlist (or, for samples, the Pattern
// Sequencer)
export type PlaceableItem = Extract<DragItem, { kind: 'sample' | 'pattern' | 'files' }>

export const isPlaceable = (item: DragItem): item is PlaceableItem =>
  item.kind === 'sample' || item.kind === 'pattern' || item.kind === 'files'

// what can be dropped on a mixer channel or into the Effects Rack
export type EffectDragItem = Extract<DragItem, { kind: 'effect' | 'slot' }>

export const isEffectItem = (item: DragItem): item is EffectDragItem =>
  item.kind === 'effect' || item.kind === 'slot'

// tells drags apart: the same key is the same thing being dragged
export const dragItemKey = (item: DragItem): string => {
  if(item.kind === 'sample') return `sample:${item.path}`
  if(item.kind === 'pattern') return `pattern:${item.patternId}`
  if(item.kind === 'effect') return `effect:${item.pluginId}`
  if(item.kind === 'instrument') return `instrument:${item.pluginId}`
  if(item.kind === 'slot') return `slot:${item.slotId}`
  if(item.kind === 'files') return `files:${item.files.length}:${item.files[0]?.path ?? ''}`
  if(item.kind === 'tab') return `tab:${item.tabKey}`
  return `folder:${item.scope}:${item.key}`
}

export const fileName = (path: string): string => path.split(/[\\/]/).pop() ?? path

// what a dragged thing is called, for the label following the mouse
export const dragItemLabel = (item: DragItem): string => {
  if(item.kind === 'sample') return fileName(item.path)
  if(item.kind === 'files') {
    if(item.files.length === 1) return fileName(item.files[0].path)
    return item.files.length ? `${item.files.length} files` : 'Nothing here plays'
  }
  return item.label
}

export interface SampleDragEvent {
  // move: the mouse moved while dragging
  // drop: the mouse was released
  // cancel: the drag was abandoned (escape pressed, or the window lost focus)
  type: 'move' | 'drop' | 'cancel'
  item: DragItem
  clientX: number
  clientY: number
}

type Listener = (ev: SampleDragEvent) => void

const listeners = new Set<Listener>()

// pixels the mouse has to move before a press becomes a drag,
// so clicking a sample to preview it doesn't start one
const DRAG_THRESHOLD = 5

// listen for sample drags, returns a function that stops listening
export const onSampleDrag = (fn: Listener) => {
  listeners.add(fn)

  return () => { listeners.delete(fn) }
}

const emit = (ev: SampleDragEvent) => listeners.forEach(fn => fn(ev))

// a drag that the window didn't start with a press: files dragged in from
// outside the app (see events/fileDrop.ts)
export const sendSampleDrag = emit

// call from a sample's mousedown. follows the mouse until it's released
export const beginSampleDrag = (
  item: DragItem,
  down: { button: number, clientX: number, clientY: number },
) => {
  if(down.button !== 0) return

  let dragging = false
  let last = { clientX: down.clientX, clientY: down.clientY }

  const { body } = document
  const prevCursor = body.style.cursor
  const prevUserSelect = body.style.userSelect

  // removes every listener added below
  const listening = new AbortController()
  const { signal } = listening

  const cleanup = () => {
    listening.abort()
    body.style.cursor = prevCursor
    body.style.userSelect = prevUserSelect
  }

  const cancel = () => {
    cleanup()
    if(dragging) emit({ type: 'cancel', item, ...last })
  }

  window.addEventListener('mousemove', (ev: MouseEvent) => {
    last = { clientX: ev.clientX, clientY: ev.clientY }

    if(!dragging) {
      const dist = Math.hypot(ev.clientX - down.clientX, ev.clientY - down.clientY)
      if(dist < DRAG_THRESHOLD) return

      dragging = true
      body.style.cursor = 'grabbing'
      // don't select text in the sidebar while dragging over it
      body.style.userSelect = 'none'
    }

    emit({ type: 'move', item, ...last })
  }, { signal })

  window.addEventListener('mouseup', (ev: MouseEvent) => {
    cleanup()
    if(dragging) emit({ type: 'drop', item, clientX: ev.clientX, clientY: ev.clientY })
  }, { signal })

  window.addEventListener('keydown', (ev: KeyboardEvent) => {
    if(ev.key === 'Escape') cancel()
  }, { signal })

  window.addEventListener('blur', cancel, { signal })
}
