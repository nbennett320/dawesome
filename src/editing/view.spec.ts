import { WHEEL_LINE_PIXELS, ZOOM_SENSITIVITY, wheelIntent, zoomAround } from './view'

const wheel = (over: Partial<Parameters<typeof wheelIntent>[0]> = {}) => ({
  deltaX: 0, deltaY: 0, deltaMode: 0, metaKey: false, ctrlKey: false, shiftKey: false, ...over,
})

describe('wheelIntent', () => {
  it('scrolls through the lanes and along time', () => {
    expect(wheelIntent(wheel({ deltaX: 3, deltaY: 12 }), 400)).toEqual({ kind: 'scroll', dx: 3, dy: 12 })
  })

  it('scrolls along time with shift held', () => {
    expect(wheelIntent(wheel({ deltaY: 12, shiftKey: true }), 400)).toEqual({ kind: 'scroll', dx: 12, dy: 0 })
    // already swapped by the system
    expect(wheelIntent(wheel({ deltaX: 12, shiftKey: true }), 400)).toEqual({ kind: 'scroll', dx: 12, dy: 0 })
  })

  it('counts a line or a page in pixels', () => {
    expect(wheelIntent(wheel({ deltaY: 2, deltaMode: 1 }), 400)).toEqual({ kind: 'scroll', dx: 0, dy: 2 * WHEEL_LINE_PIXELS })
    expect(wheelIntent(wheel({ deltaY: 1, deltaMode: 2 }), 400)).toEqual({ kind: 'scroll', dx: 0, dy: 400 })
  })

  it('zooms with ⌘ or ⌃ held: in when scrolled up, out when down', () => {
    const zoomIn = wheelIntent(wheel({ deltaY: -2, metaKey: true }), 400)
    const zoomOut = wheelIntent(wheel({ deltaY: 2, ctrlKey: true }), 400)
    expect(zoomIn.kind).toBe('zoom')
    expect(zoomIn.kind === 'zoom' && zoomIn.factor).toBeGreaterThan(1)
    expect(zoomOut.kind === 'zoom' && zoomOut.factor).toBeLessThan(1)
  })

  it('zooms by no more than a little per event', () => {
    const fast = wheelIntent(wheel({ deltaY: -500, metaKey: true }), 400)
    expect(fast).toEqual({ kind: 'zoom', factor: 1 + ZOOM_SENSITIVITY })
    const back = wheelIntent(wheel({ deltaY: 500, metaKey: true }), 400)
    expect(back).toEqual({ kind: 'zoom', factor: 1 - ZOOM_SENSITIVITY })
  })
})

describe('zoomAround', () => {
  it('keeps the point under the mouse where it is', () => {
    // content x 150 is under the mouse at screen x 100 (pan -200, scale 2)
    const before = { scale: 2, pan: -200 }
    const after = zoomAround(before, 1.5, 100, 1)
    expect(after.scale).toBe(3)
    expect((100 - after.pan) / after.scale).toBeCloseTo((100 - before.pan) / before.scale)
  })

  it('stops at the least and the most zoom', () => {
    expect(zoomAround({ scale: 1.2, pan: -10 }, 0.5, 0, 1).scale).toBe(1)
    expect(zoomAround({ scale: 3, pan: -10 }, 2, 0, 1, 4).scale).toBe(4)
  })

  it('ignores a factor that makes no sense', () => {
    const zoom = { scale: 2, pan: -5 }
    expect(zoomAround(zoom, 0, 10, 1)).toEqual(zoom)
    expect(zoomAround(zoom, NaN, 10, 1)).toEqual(zoom)
  })
})
