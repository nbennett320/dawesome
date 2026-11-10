// gestures: one press of the mouse, from the button going down to it coming
// up. everything a gesture changes (a fader dragged from one place to
// another, a row of steps painted in one stroke) is undone as one step: each
// change is sent to the engine with the gesture's id, and the engine folds
// changes with the same id together (see engine/src/history.rs)
//
// changes made without the mouse (a key press) belong to no gesture, and are
// a step each

let nextId = 1
let current: number | null = null
// counts presses, so a release only ends the press it belongs to
let presses = 0

// the gesture in progress, if any: sent with each change made during it
export const currentGesture = (): number | null => current

// whether the mouse is held down. undo and redo wait until it's let go
export const gestureActive = (): boolean => current !== null

// a new gesture, for something that isn't a press of the mouse (a visit to
// the tempo field): its id is passed with each of its changes
export const newGesture = (): number => {
  nextId += 1
  return nextId - 1
}

export const beginGesture = (): number => {
  current = newGesture()
  presses += 1
  return current
}

// the mouse came up. the gesture ends a moment later, so whatever the
// release itself does (a clip dropped where it was dragged to) is still
// part of it
export const endGesture = (): void => {
  const press = presses
  setTimeout(() => {
    if(press === presses) current = null
  }, 0)
}

// for tests
export const resetGestures = (): void => {
  current = null
  presses = 0
}

if(typeof window !== 'undefined') {
  // as the events go down (capture), so the gesture has begun before
  // anything acts on the press
  window.addEventListener('pointerdown', () => { beginGesture() }, true)
  window.addEventListener('pointerup', endGesture, true)
  window.addEventListener('pointercancel', endGesture, true)
  // the mouse came up outside the window
  window.addEventListener('blur', endGesture)
  // a release can go missing (a context menu takes it): the mouse moving
  // with no button down means the press is over
  window.addEventListener('contextmenu', endGesture, true)
  window.addEventListener('pointermove', (ev: PointerEvent) => {
    if(current !== null && ev.buttons === 0) endGesture()
  }, true)
}
