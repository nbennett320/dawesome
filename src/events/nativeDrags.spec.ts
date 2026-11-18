import { stopNativeDrags } from './nativeDrags'

const dragStart = (from: EventTarget) => {
  const ev = new Event('dragstart', { bubbles: true, cancelable: true })
  from.dispatchEvent(ev)
  return ev
}

describe('stopNativeDrags', () => {
  it('stops the browser dragging anything on the page, until undone', () => {
    const picture = document.body.appendChild(document.createElement('img'))
    const undo = stopNativeDrags()
    expect(dragStart(picture).defaultPrevented).toBe(true)
    expect(dragStart(document.body).defaultPrevented).toBe(true)
    undo()
    expect(dragStart(picture).defaultPrevented).toBe(false)
    picture.remove()
  })
})
