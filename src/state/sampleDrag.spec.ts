import { beginSampleDrag } from './sampleDrag'

const item = { kind: 'tab' as const, tabKey: 'playlist', label: 'Playlist' }
const press = () => beginSampleDrag(item, { button: 0, clientX: 0, clientY: 0 })
const selectStart = () => {
  const ev = new Event('selectstart', { cancelable: true })
  window.dispatchEvent(ev)
  return ev
}

describe('a drag on the drag bus', () => {
  afterEach(() => window.dispatchEvent(new MouseEvent('mouseup')))

  it('selects nothing while the button is down, and selects again after', () => {
    press()
    expect(selectStart().defaultPrevented).toBe(true)
    window.dispatchEvent(new MouseEvent('mouseup'))
    expect(selectStart().defaultPrevented).toBe(false)
  })

  it('lets go of what was selected as it begins', () => {
    const label = document.createElement('span')
    label.textContent = 'Playlist'
    document.body.append(label)
    window.getSelection()?.selectAllChildren(label)
    press()
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 20, clientY: 0 }))
    expect(window.getSelection()?.toString()).toBe('')
    expect(document.body.style.userSelect).toBe('none')
    window.dispatchEvent(new MouseEvent('mouseup'))
    expect(document.body.style.userSelect).toBe('')
    label.remove()
  })
})
