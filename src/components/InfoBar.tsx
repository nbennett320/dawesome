import React from 'react'
import { useAppDispatch, useAppSelector } from '../hooks/redux'
import { selectAppearance } from '../state/slices/appearanceSlice'
import { findCommand } from '../state/commands'
import { isTypingTarget } from '../util/keys'

// how long a message (e.g. an unknown command) stays before hints come back
const MESSAGE_MS = 3000

// the strip along the bottom: describes whatever the mouse is over. anything
// can offer a description with a data-hint attribute, or falls back on its
// title (tooltip); the nearest one up from the hovered element wins.
//
// with the command bar on (Preferences › Controls), typing `:` (outside a
// text field) turns it into a command line, as in vim:
// Enter runs the command (see state/commands.ts), Esc puts it away
const InfoBar = () => {
  const dispatch = useAppDispatch()
  const enabled = useAppSelector(selectAppearance).preferences.commandBar
  const [hint, setHint] = React.useState('')
  // the command being typed, while the command line is open
  const [line, setLine] = React.useState<string | null>(null)
  const [message, setMessage] = React.useState('')
  const input = React.useRef<HTMLInputElement>(null)

  // the command line takes the keyboard as it opens
  const open = line !== null
  React.useEffect(() => { if(open) input.current?.focus() }, [open])

  React.useEffect(() => {
    const onOver = (e: MouseEvent) => {
      const el = (e.target as Element | null)?.closest?.('[data-hint], [title]')
      setHint(el?.getAttribute('data-hint') ?? el?.getAttribute('title') ?? '')
    }
    // leaving the window clears it
    const onLeave = () => setHint('')

    document.addEventListener('mouseover', onOver)
    document.documentElement.addEventListener('mouseleave', onLeave)
    return () => {
      document.removeEventListener('mouseover', onOver)
      document.documentElement.removeEventListener('mouseleave', onLeave)
    }
  }, [])

  // `:` opens the command line
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if(e.key !== ':' || e.metaKey || e.ctrlKey || e.altKey || isTypingTarget(e.target)) return
      e.preventDefault()
      setMessage('')
      setLine('')
    }
    if(!enabled) return undefined
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [enabled])

  // turned off while open: put it away
  React.useEffect(() => { if(!enabled) setLine(null) }, [enabled])

  React.useEffect(() => {
    if(!message) return undefined
    const timer = setTimeout(() => setMessage(''), MESSAGE_MS)
    return () => clearTimeout(timer)
  }, [message])

  const run = (typed: string) => {
    setLine(null)
    if(!typed.trim()) return
    const command = findCommand(typed)
    if(!command) {
      setMessage(`Not a command: ${typed.trim()}`)
      return
    }
    Promise.resolve(command(dispatch))
      .then(why => { if(typeof why === 'string') setMessage(why) })
      // eslint-disable-next-line no-console
      .catch(e => console.error(`:${typed.trim()} failed:`, e))
  }

  if(line !== null) {
    return (
      <div className="InfoBar flex items-center px-3 border-t border-line bg-panel text-xs text-ink-strong font-mono">
        <span>:</span>
        <input
          ref={input}
          aria-label='Command'
          className='flex-1 bg-transparent outline-none'
          value={line}
          spellCheck={false}
          onChange={e => setLine(e.target.value)}
          onBlur={() => setLine(null)}
          onKeyDown={e => {
            if(e.key === 'Enter') run(line)
            else if(e.key === 'Escape') setLine(null)
            // backspacing past the `:` closes it, as in vim
            else if(e.key === 'Backspace' && line === '') setLine(null)
          }}
        />
      </div>
    )
  }

  return (
    <div className={`InfoBar flex items-center px-3 border-t border-line bg-panel text-xs truncate ${message ? 'text-danger' : 'text-ink-soft'}`}>
      {message || hint}
    </div>
  )
}

export default InfoBar
