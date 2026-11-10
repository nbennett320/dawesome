// whether a key press is going into something the user types in (a text
// box, a number field, an editable element), so app shortcuts leave it alone
export const isTypingTarget = (target: EventTarget | null): boolean => {
  if(!(target instanceof HTMLElement)) return false
  if(target.isContentEditable) return true

  const tag = target.tagName
  if(tag === 'TEXTAREA' || tag === 'SELECT') return true
  if(tag !== 'INPUT') return false

  // buttons, checkboxes and sliders don't take typing
  const { type } = target as HTMLInputElement
  return !['button', 'checkbox', 'radio', 'range', 'submit', 'reset', 'color', 'file'].includes(type)
}

// Delete, or Backspace (the delete key on a Mac keyboard), with no modifier
export const isDeleteKey = (ev: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey'>): boolean =>
  (ev.key === 'Delete' || ev.key === 'Backspace') && !ev.metaKey && !ev.ctrlKey && !ev.altKey

// Shift+E (explode the selected pattern clips), with no other modifier.
// with shift held the key is "E", but some layouts report "e"
export const isExplodeKey = (
  ev: Pick<KeyboardEvent, 'key' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey'>,
): boolean =>
  ev.key.toLowerCase() === 'e' && ev.shiftKey && !ev.metaKey && !ev.ctrlKey && !ev.altKey

// Space with no modifier: the transport's key (⌘ and ⌃ Space belong to the
// system)
export const isTransportKey = (
  ev: Pick<KeyboardEvent, 'key' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey'>,
): boolean =>
  ev.key === ' ' && !ev.shiftKey && !ev.metaKey && !ev.ctrlKey && !ev.altKey

type TransportKeyEvent = Pick<
  KeyboardEvent,
  'type' | 'key' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey' | 'repeat' | 'target'
  | 'preventDefault' | 'stopPropagation'
>

// what Space does, anywhere in the window: play or pause (`onToggle`), once
// per press. the key is kept from whatever has the keyboard (a focused
// button would be clicked by it, on the way down or up), except something
// that takes typing, which gets its space as usual
//
// for `keydown` and `keyup`, listened to on the window as the event goes
// down (capture), so it runs before any control's own handler
export const transportKeyHandler = (onToggle: () => void) => (ev: TransportKeyEvent): void => {
  if(!isTransportKey(ev) || isTypingTarget(ev.target)) return

  ev.preventDefault()
  ev.stopPropagation()
  // held down, it toggles once
  if(ev.type === 'keydown' && !ev.repeat) onToggle()
}
