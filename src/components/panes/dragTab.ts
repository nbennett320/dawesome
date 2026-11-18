import React from 'react'
import { DragItem, onSampleDrag } from '../../state/sampleDrag'

// dragging a pane's tab: to another pane's tabs, or onto a pane's body to
// split it. on the drag bus's plain mouse events (see state/sampleDrag.ts),
// not HTML drag and drop: Tauri's file drop handler (dragDropEnabled, for
// files dragged in from Finder) takes every HTML drag for itself, so the page
// would never hear of one

export type TabDragItem = Extract<DragItem, { kind: 'tab' }>

// whether a tab is being dragged, e.g. to hide what would cover the panes'
// drop zones
export const useDraggingTab = (): boolean => {
  const [dragging, setDragging] = React.useState(false)
  React.useEffect(() => onSampleDrag(({ type, item }) => {
    setDragging(type === 'move' && item.kind === 'tab')
  }), [])
  return dragging
}

interface TabDropOptions {
  // the tab moved over the element, at this point in the window
  hover?: (item: TabDragItem, clientX: number, clientY: number) => void
  drop: (item: TabDragItem, clientX: number, clientY: number) => void
}

// somewhere a tab can be dropped: `ref`'s element, when what's under the
// mouse is it or inside it. returns whether a tab is over it
export const useTabDrop = (ref: React.RefObject<HTMLElement>, options: TabDropOptions): boolean => {
  const [over, setOver] = React.useState(false)
  const latest = React.useRef(options)
  latest.current = options

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    const el = ref.current
    const inside = item.kind === 'tab' && !!el && el.contains(document.elementFromPoint(clientX, clientY))
    if(type === 'move') {
      setOver(inside)
      if(inside) latest.current.hover?.(item as TabDragItem, clientX, clientY)
      return
    }
    setOver(false)
    if(type === 'drop' && inside) latest.current.drop(item as TabDragItem, clientX, clientY)
  }), [ref])

  return over
}
