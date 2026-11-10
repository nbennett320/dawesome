import {
  isDeleteKey, isExplodeKey, isTransportKey, isTypingTarget, transportKeyHandler,
} from './keys'

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

describe('isExplodeKey', () => {
  const key = (k: string, mods: Partial<KeyboardEvent> = {}) =>
    ({ key: k, shiftKey: true, metaKey: false, ctrlKey: false, altKey: false, ...mods })

  it('is Shift+E', () => {
    expect(isExplodeKey(key('E'))).toBe(true)
    expect(isExplodeKey(key('e'))).toBe(true)
  })

  it('is not E without shift, or with another modifier', () => {
    expect(isExplodeKey(key('e', { shiftKey: false }))).toBe(false)
    expect(isExplodeKey(key('E', { metaKey: true }))).toBe(false)
    expect(isExplodeKey(key('R'))).toBe(false)
  })
})

describe('the transport key', () => {
  const none = { shiftKey: false, metaKey: false, ctrlKey: false, altKey: false }
  const press = (over: Partial<KeyboardEvent> = {}) => ({
    type: 'keydown',
    key: ' ',
    ...none,
    repeat: false,
    target: document.body as EventTarget | null,
    preventDefault: jest.fn(),
    stopPropagation: jest.fn(),
    ...over,
  })

  it('is Space on its own', () => {
    expect(isTransportKey({ key: ' ', ...none })).toBe(true)
    expect(isTransportKey({ key: 'Enter', ...none })).toBe(false)
    // ⌘ Space and ⌃ Space are the system's
    expect(isTransportKey({ key: ' ', ...none, metaKey: true })).toBe(false)
    expect(isTransportKey({ key: ' ', ...none, ctrlKey: true })).toBe(false)
    expect(isTransportKey({ key: ' ', ...none, shiftKey: true })).toBe(false)
    expect(isTransportKey({ key: ' ', ...none, altKey: true })).toBe(false)
  })

  it('plays or pauses, and keeps the key from what has the keyboard', () => {
    const toggle = jest.fn()
    // a focused button would otherwise be clicked by it
    const ev = press({ target: el('<button></button>') })
    transportKeyHandler(toggle)(ev)

    expect(toggle).toHaveBeenCalledTimes(1)
    expect(ev.preventDefault).toHaveBeenCalled()
    expect(ev.stopPropagation).toHaveBeenCalled()
  })

  it('toggles once while held, and not again when let go', () => {
    const toggle = jest.fn()
    const handle = transportKeyHandler(toggle)
    handle(press())
    const held = press({ repeat: true })
    handle(held)
    handle(held)
    const up = press({ type: 'keyup' })
    handle(up)

    expect(toggle).toHaveBeenCalledTimes(1)
    // still kept from a focused button, which clicks on the way up
    expect(held.preventDefault).toHaveBeenCalledTimes(2)
    expect(up.preventDefault).toHaveBeenCalled()
  })

  it('types a space wherever typing goes', () => {
    const toggle = jest.fn()
    const handle = transportKeyHandler(toggle)
    const fields = [
      '<input type="text">', '<input type="number">', '<input type="search">', '<input>',
      '<textarea></textarea>', '<select><option>1/4</option></select>',
    ].map(el)
    const editable = el('<div contenteditable="true"></div>')
    Object.defineProperty(editable, 'isContentEditable', { value: true })

    ;[...fields, editable].forEach(target => {
      const ev = press({ target })
      handle(ev)
      handle(press({ type: 'keyup', target }))
      expect(ev.preventDefault).not.toHaveBeenCalled()
      expect(ev.stopPropagation).not.toHaveBeenCalled()
    })
    expect(toggle).not.toHaveBeenCalled()
  })

  it('leaves other keys alone', () => {
    const toggle = jest.fn()
    const ev = press({ key: 'Enter' })
    transportKeyHandler(toggle)(ev)
    const cmd = press({ metaKey: true })
    transportKeyHandler(toggle)(cmd)

    expect(toggle).not.toHaveBeenCalled()
    expect(ev.preventDefault).not.toHaveBeenCalled()
    expect(cmd.preventDefault).not.toHaveBeenCalled()
  })
})
