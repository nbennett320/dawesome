import React from 'react'
import { visibleBars } from './stepGrid'

export interface VisibleBars {
  // the bars to render, from `from` up to `to` (see visibleBars)
  from: number
  to: number
  // how wide the scroller's view is, in px
  width: number
}

// the bars of the step grid in a scroller's view, and one more either side,
// followed as it scrolls and is resized. `left` is where the grid starts in
// what it scrolls, `bars` how many it has. a new answer only when the bars
// (or the width) change, so scrolling within a bar renders nothing
export const useVisibleBars = (
  scroller: React.RefObject<HTMLElement>, left: number, beatsPerBar: number, bars: number,
): VisibleBars => {
  const [shown, setShown] = React.useState<VisibleBars>({ from: 0, to: 0, width: 0 })

  // (before the rows are painted, so they never show an empty range)
  React.useLayoutEffect(() => {
    const el = scroller.current
    if(!el) return undefined
    const follow = () => {
      const width = el.clientWidth
      const { from, to } = visibleBars(el.scrollLeft - left, width, beatsPerBar, bars)
      setShown(now => (now.from === from && now.to === to && now.width === width ? now : { from, to, width }))
    }
    follow()
    el.addEventListener('scroll', follow, { passive: true })
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(follow)
    observer?.observe(el)
    return () => {
      el.removeEventListener('scroll', follow)
      observer?.disconnect()
    }
  }, [scroller, left, beatsPerBar, bars])

  return shown
}
