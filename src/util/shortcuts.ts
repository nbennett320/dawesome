// the app's ⌘ shortcuts (⌃ on other systems), as seen by the window
//
// the Edit menu has the same shortcuts (see build_menu in
// engine/src/app/window.rs). whether a press reaches the menu, the window,
// or both depends on the system, so both are listened to, and a press that
// arrives twice is acted on once (see samePress in state/history.ts)
import { isTextTarget } from './keys'

export const isMac = (): boolean =>
  typeof navigator !== 'undefined' && /mac/i.test(`${navigator.platform} ${navigator.userAgent}`)

type KeyPress = Pick<KeyboardEvent, 'key' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey'>

// ⌘ on a Mac, and only ⌘ (⌃C isn't copy there). ⌃ everywhere else
export const commandHeld = (ev: KeyPress, mac: boolean = isMac()): boolean =>
  (mac ? ev.metaKey && !ev.ctrlKey : ev.ctrlKey && !ev.metaKey) && !ev.altKey

export type EditShortcut = 'undo' | 'redo'

// the edit command a key press is the shortcut for, if any. letters are
// matched by the character the key types, whatever the layout
export const editShortcut = (ev: KeyPress, mac: boolean = isMac()): EditShortcut | null => {
  if(!commandHeld(ev, mac)) return null

  switch(ev.key.toLowerCase()) {
    case 'z': return ev.shiftKey ? 'redo' : 'undo'
    default: return null
  }
}

type ShortcutEvent = KeyPress & Pick<KeyboardEvent, 'target' | 'preventDefault'>

// for `keydown` on the window: run the edit command a press is the shortcut
// for. in a text field the key is left alone, and does what it does to
// text. (a drop-down that still has the keyboard after a pick isn't one)
export const editShortcutHandler = (
  run: (command: EditShortcut) => void,
  mac: boolean = isMac(),
) => (ev: ShortcutEvent): void => {
  const command = editShortcut(ev, mac)
  if(!command || isTextTarget(ev.target)) return

  ev.preventDefault()
  run(command)
}
