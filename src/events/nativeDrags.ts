// the browser's own drags, of selected text, an image or a link, never start:
// nothing in the app takes them (its drags are on plain mouse events, see
// state/sampleDrag.ts), and Tauri's file drop handler swallows them anyway,
// so all one did was drag a picture of the page around. files dragged in
// from Finder start outside the page, so they still come in. returns what
// lets them start again
export const stopNativeDrags = (target: Pick<Window, 'addEventListener' | 'removeEventListener'> = window) => {
  const stop = (ev: Event) => ev.preventDefault()
  target.addEventListener('dragstart', stop)
  return () => target.removeEventListener('dragstart', stop)
}
