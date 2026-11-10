import {
  beginGesture, currentGesture, endGesture, gestureActive, newGesture, resetGestures,
} from './gesture'

describe('gestures', () => {
  beforeEach(() => {
    jest.useFakeTimers()
    resetGestures()
  })
  afterEach(() => jest.useRealTimers())

  it('lasts from the press to just after the release', () => {
    expect(currentGesture()).toBeNull()
    expect(gestureActive()).toBe(false)

    const id = beginGesture()
    expect(currentGesture()).toBe(id)
    expect(gestureActive()).toBe(true)

    // what the release itself does is still part of it
    endGesture()
    expect(currentGesture()).toBe(id)
    jest.runAllTimers()
    expect(currentGesture()).toBeNull()
  })

  it('gives every press its own id', () => {
    const first = beginGesture()
    endGesture()
    jest.runAllTimers()
    const second = beginGesture()

    expect(second).not.toBe(first)
    expect(newGesture()).not.toBe(second)
  })

  it('is not ended by the last press letting go late', () => {
    beginGesture()
    endGesture()
    // pressed again before the first release's moment has passed
    const second = beginGesture()
    jest.runAllTimers()

    expect(currentGesture()).toBe(second)
  })

  it('follows the mouse in the window', () => {
    window.dispatchEvent(new Event('pointerdown'))
    expect(gestureActive()).toBe(true)
    window.dispatchEvent(new Event('pointerup'))
    jest.runAllTimers()
    expect(gestureActive()).toBe(false)
  })
})
