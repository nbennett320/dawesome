import { commandHeld, editShortcut, editShortcutHandler } from './shortcuts'

const none = { shiftKey: false, metaKey: false, ctrlKey: false, altKey: false }
const cmd = { ...none, metaKey: true }
const ctrl = { ...none, ctrlKey: true }

describe('the command key', () => {
  it('is ⌘ on a Mac, and only ⌘', () => {
    expect(commandHeld({ key: 'z', ...cmd }, true)).toBe(true)
    expect(commandHeld({ key: 'z', ...ctrl }, true)).toBe(false)
    expect(commandHeld({ key: 'z', ...cmd, ctrlKey: true }, true)).toBe(false)
    expect(commandHeld({ key: 'z', ...none }, true)).toBe(false)
  })

  it('is ⌃ everywhere else', () => {
    expect(commandHeld({ key: 'z', ...ctrl }, false)).toBe(true)
    expect(commandHeld({ key: 'z', ...cmd }, false)).toBe(false)
  })

  it('is not held with ⌥', () => {
    expect(commandHeld({ key: 'z', ...cmd, altKey: true }, true)).toBe(false)
  })
})

describe('edit shortcuts', () => {
  it('⌘Z undoes and ⇧⌘Z redoes', () => {
    expect(editShortcut({ key: 'z', ...cmd }, true)).toBe('undo')
    expect(editShortcut({ key: 'Z', ...cmd, shiftKey: true }, true)).toBe('redo')
    // caps lock on
    expect(editShortcut({ key: 'Z', ...cmd }, true)).toBe('undo')
    expect(editShortcut({ key: 'z', ...ctrl }, false)).toBe('undo')
  })

  it('needs the command key', () => {
    expect(editShortcut({ key: 'z', ...none }, true)).toBeNull()
    // on a Mac, ⌃Z does nothing to the project
    expect(editShortcut({ key: 'z', ...ctrl }, true)).toBeNull()
    expect(editShortcut({ key: 'y', ...cmd }, true)).toBeNull()
  })
})

describe('editShortcutHandler', () => {
  const press = (over: object = {}) => ({
    key: 'z', ...cmd, target: document.body as EventTarget | null, preventDefault: jest.fn(), ...over,
  })

  it('runs the command, and keeps the key from the window', () => {
    const run = jest.fn()
    const ev = press()
    editShortcutHandler(run, true)(ev)

    expect(run).toHaveBeenCalledWith('undo')
    expect(ev.preventDefault).toHaveBeenCalled()
  })

  it('still runs with a drop-down or a button holding the keyboard', () => {
    // a drop-down keeps the keyboard after a pick, and has no text to undo
    const run = jest.fn()
    ;['select', 'button', 'canvas'].forEach(tag => {
      const ev = press({ target: document.createElement(tag) })
      editShortcutHandler(run, true)(ev)
      expect(ev.preventDefault).toHaveBeenCalled()
    })
    expect(run).toHaveBeenCalledTimes(3)
  })

  it('leaves the key to a text field, where it undoes typing', () => {
    const run = jest.fn()
    const field = document.createElement('input')
    const ev = press({ target: field })
    editShortcutHandler(run, true)(ev)

    expect(run).not.toHaveBeenCalled()
    expect(ev.preventDefault).not.toHaveBeenCalled()
  })

  it('ignores other keys', () => {
    const run = jest.fn()
    const ev = press({ key: 'q' })
    editShortcutHandler(run, true)(ev)

    expect(run).not.toHaveBeenCalled()
    expect(ev.preventDefault).not.toHaveBeenCalled()
  })
})
