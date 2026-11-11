import {
  beginGesture, currentGesture, endGesture, gestureActive, newGesture, resetGestures,
} from './gesture'

describe('gestures', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    resetGestures()
  })
  afterEach(() => vi.useRealTimers())

  it('lasts from the press to just after the release', () => {
    expect(currentGesture()).toBeNull()
    expect(gestureActive()).toBe(false)

    const id = beginGesture()
    expect(currentGesture()).toBe(id)
    expect(gestureActive()).toBe(true)

    // what the release itself does is still part of it
    endGesture()
    expect(currentGesture()).toBe(id)
    vi.runAllTimers()
    expect(currentGesture()).toBeNull()
  })

  it('gives every press its own id', () => {
    const first = beginGesture()
    endGesture()
    vi.runAllTimers()
    const second = beginGesture()

    expect(second).not.toBe(first)
    expect(newGesture()).not.toBe(second)
  })

  it('is not ended by the last press letting go late', () => {
    beginGesture()
    endGesture()
    // pressed again before the first release's moment has passed
    const second = beginGesture()
    vi.runAllTimers()

    expect(currentGesture()).toBe(second)
  })

  it('follows the mouse in the window', () => {
    window.dispatchEvent(new Event('pointerdown'))
    expect(gestureActive()).toBe(true)
    window.dispatchEvent(new Event('pointerup'))
    vi.runAllTimers()
    expect(gestureActive()).toBe(false)
  })

  it('ends when the release went missing: a context menu, or the mouse moving with no button down', () => {
    window.dispatchEvent(new Event('pointerdown'))
    window.dispatchEvent(new Event('contextmenu'))
    vi.runAllTimers()
    expect(gestureActive()).toBe(false)

    const move = (buttons: number) => Object.assign(new Event('pointermove'), { buttons })
    window.dispatchEvent(new Event('pointerdown'))
    // still dragging
    window.dispatchEvent(move(1))
    vi.runAllTimers()
    expect(gestureActive()).toBe(true)
    window.dispatchEvent(move(0))
    vi.runAllTimers()
    expect(gestureActive()).toBe(false)
  })
})
