import { isDeleteKey, isTypingTarget } from './keys'

const el = (html: string) => {
  const box = document.createElement('div')
  box.innerHTML = html
  return box.firstElementChild as HTMLElement
}

describe('isTypingTarget', () => {
  it('is true for text boxes and editable elements', () => {
    expect(isTypingTarget(el('<input type="text">'))).toBe(true)
    expect(isTypingTarget(el('<input type="number">'))).toBe(true)
    expect(isTypingTarget(el('<input>'))).toBe(true)
    expect(isTypingTarget(el('<textarea></textarea>'))).toBe(true)
    const editable = el('<div contenteditable="true"></div>')
    // jsdom doesn't work out isContentEditable from the attribute
    Object.defineProperty(editable, 'isContentEditable', { value: true })
    expect(isTypingTarget(editable)).toBe(true)
  })

  it('is false for the page, the canvas, buttons and sliders', () => {
    expect(isTypingTarget(null)).toBe(false)
    expect(isTypingTarget(document.body)).toBe(false)
    expect(isTypingTarget(el('<canvas></canvas>'))).toBe(false)
    expect(isTypingTarget(el('<button></button>'))).toBe(false)
    expect(isTypingTarget(el('<input type="range">'))).toBe(false)
    expect(isTypingTarget(el('<input type="checkbox">'))).toBe(false)
  })
})

describe('isDeleteKey', () => {
  const key = (k: string, mods: Partial<KeyboardEvent> = {}) =>
    ({ key: k, metaKey: false, ctrlKey: false, altKey: false, ...mods })

  it('is Delete or Backspace on its own', () => {
    expect(isDeleteKey(key('Delete'))).toBe(true)
    expect(isDeleteKey(key('Backspace'))).toBe(true)
  })

  it('is not another key, or a shortcut with a modifier', () => {
    expect(isDeleteKey(key('d'))).toBe(false)
    expect(isDeleteKey(key('Backspace', { metaKey: true }))).toBe(false)
    expect(isDeleteKey(key('Delete', { altKey: true }))).toBe(false)
  })
})
