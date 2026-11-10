import React from 'react'
import { useAppDispatch } from '../hooks/redux'
import { EditCommand, listeners } from '../events/window'
import { isTypingTarget } from '../util/keys'
import { EditShortcut, editShortcutHandler } from '../util/shortcuts'
import { travel } from './history'

// the Edit menu and its shortcuts, wherever in the window the keyboard is.
// a shortcut reaches here from the menu, from the key press itself, or both
// (see util/shortcuts.ts)
export const useEditCommands = () => {
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    // from the menu: the key (if it was a key) never reached the window, so
    // in a text field the command is done to the text here
    const fromMenu = (command: EditCommand) => {
      const typing = isTypingTarget(document.activeElement)

      switch(command) {
        case 'undo':
        case 'redo':
          if(typing) document.execCommand(command)
          else dispatch(travel(command, 'menu'))
          break
        case 'select-all':
          if(typing) document.execCommand('selectAll')
          break
        default:
      }
    }

    // from the key: a text field keeps it (see editShortcutHandler)
    const onKey = editShortcutHandler((command: EditShortcut) => {
      dispatch(travel(command, 'key'))
    })

    const menu = listeners.editMenu(fromMenu)
    window.addEventListener('keydown', onKey)
    return () => {
      menu.then(stop => stop())
      window.removeEventListener('keydown', onKey)
    }
  }, [dispatch])
}
