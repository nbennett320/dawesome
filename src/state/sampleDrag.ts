// dragging a sample from the sidebar into the playlist or the sequencer,
// or a pattern from the Sequencer's pattern strip into the playlist
//
// this uses plain mouse events instead of HTML drag and drop (react-dnd):
// during an HTML drag the playlist canvas couldn't draw the clip following
// the mouse, while mouse events keep p5's draw loop running as normal

// what's being dragged
export type DragItem =
  | { kind: 'sample', path: string }
  | { kind: 'pattern', patternId: number, label: string }

// tells drags apart: the same key is the same thing being dragged
export const dragItemKey = (item: DragItem): string =>
  (item.kind === 'sample' ? `sample:${item.path}` : `pattern:${item.patternId}`)

// what a dragged thing is called, for the label following the mouse
export const dragItemLabel = (item: DragItem): string =>
  (item.kind === 'sample' ? item.path.split(/[\\/]/).pop() ?? item.path : item.label)

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
