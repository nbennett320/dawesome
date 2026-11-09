import React from 'react'

// pixels the mouse can move between press and release and still count as a
// click (the same as a drag's threshold in state/sampleDrag.ts)
const CLICK_SLOP = 5

// for a row that can be dragged and clicked: call `pressed` from its
// mousedown, and `isClick` from its click, which is false when the press
// became a drag (or for a click from the keyboard, true)
export const useClickWithoutDrag = () => {
  const down = React.useRef<{ x: number, y: number } | null>(null)

  const pressed = React.useCallback((ev: React.MouseEvent) => {
    down.current = { x: ev.clientX, y: ev.clientY }
  }, [])

  const isClick = React.useCallback((ev: React.MouseEvent) => {
    const from = down.current
    down.current = null
    // detail is 0 for a click from the keyboard
    if(!ev.detail || !from) return true
    return Math.hypot(ev.clientX - from.x, ev.clientY - from.y) < CLICK_SLOP
  }, [])

  return { pressed, isClick }
}
