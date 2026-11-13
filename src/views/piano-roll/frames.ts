// drawing at most once a frame. whatever changes what a canvas shows asks
// for it to be drawn again (redraw), and at the next frame each canvas
// asked for is drawn once, with what's latest then, however many times it
// was asked for since: a drag, a scroll and a selection in the same frame
// are one drawing, not three
export interface FrameDrawer<L> {
  redraw: (...layers: L[]) => void
  // nothing more is drawn (the canvases are going)
  cancel: () => void
}

export const frameDrawer = <L>(draw: (layers: ReadonlySet<L>) => void): FrameDrawer<L> => {
  let asked = new Set<L>()
  let frame = 0

  return {
    redraw: (...layers) => {
      layers.forEach(layer => asked.add(layer))
      if(frame) return
      frame = requestAnimationFrame(() => {
        frame = 0
        const now = asked
        asked = new Set()
        draw(now)
      })
    },
    cancel: () => {
      cancelAnimationFrame(frame)
      frame = 0
      asked = new Set()
    },
  }
}
