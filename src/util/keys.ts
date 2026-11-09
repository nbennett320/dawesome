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
