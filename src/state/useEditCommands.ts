import React from 'react'
import { useAppDispatch } from '../hooks/redux'
import { EditCommand, listeners } from '../events/window'
import { isTextTarget } from '../util/keys'
import { editShortcut, editShortcutHandler } from '../util/shortcuts'
import { travel } from './history'
import { runClipCommandFromMenu } from './clipCommands'

// the Edit menu and its shortcuts, wherever in the window the keyboard is.
// a shortcut reaches here from the menu, from the key press itself, or both
// (see util/shortcuts.ts)
export const useEditCommands = () => {
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    // from the menu: the key (if it was a key) never reached the window, so
    // in a text field the command is done to the text here
    const fromMenu = (command: EditCommand) => {
      const typing = isTextTarget(document.activeElement)

      switch(command) {
        case 'undo':
        case 'redo':
          if(typing) document.execCommand(command)
          else dispatch(travel(command, 'menu'))
          break
        case 'repeat':
          if(!typing) runClipCommandFromMenu('repeat')
          break
        case 'join':
          if(!typing) runClipCommandFromMenu('join')
          break
        case 'select-all':
          if(typing) document.execCommand('selectAll')
          else runClipCommandFromMenu('select-all')
          break
        default:
      }
    }

    // from the key: a text field keeps it (see editShortcutHandler). the
    // other commands are the Playlist's own (see PlaylistCanvas), but ⌘R is
    // kept from reloading the window here, whichever view has the keyboard
    const handle = editShortcutHandler(command => {
      if(command === 'undo' || command === 'redo') dispatch(travel(command, 'key'))
    })
    const onKey = (ev: KeyboardEvent) => {
      const command = editShortcut(ev)
      if(command === 'undo' || command === 'redo' || command === 'repeat') handle(ev)
    }

    const menu = listeners.editMenu(fromMenu)
    window.addEventListener('keydown', onKey)
    return () => {
      menu.then(stop => stop())
      window.removeEventListener('keydown', onKey)
    }
  }, [dispatch])
}
