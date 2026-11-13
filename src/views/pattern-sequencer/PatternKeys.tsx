import React from 'react'
import { createPortal } from 'react-dom'
import { Pattern, patternLabel, searchPatterns } from '../../types/sequencer'
import { beginSampleDrag, onSampleDrag } from '../../state/sampleDrag'
import { AnalogButton, Led } from '../../components/fourtrack'

// how wide the fade at a scrolled edge of the keys is, in px
const FADE = 18

// which ends of a scrolling box have more past them
const overflowOf = (el: HTMLElement | null) => ({
  left: !!el && el.scrollLeft > 1,
  right: !!el && el.scrollLeft + el.clientWidth < el.scrollWidth - 1,
})

// the pattern keys in one line, scrolled sideways when they don't fit (the
// mouse wheel scrolls them too), faded at an end with more past it. the
// selected key is scrolled into view when it changes
export const ScrollingKeys = ({ selectedId, children }: {
  selectedId: number | null
  children: React.ReactNode
}) => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [more, setMore] = React.useState({ left: false, right: false })

  const measure = React.useCallback(() => {
    const next = overflowOf(ref.current)
    setMore(now => (now.left === next.left && now.right === next.right ? now : next))
  }, [])

  React.useEffect(() => {
    const el = ref.current
    if(!el) return undefined
    measure()
    // an up-and-down wheel scrolls sideways (passive: false, to keep the
    // page from scrolling as well)
    const wheel = (ev: WheelEvent) => {
      if(Math.abs(ev.deltaY) <= Math.abs(ev.deltaX) || el.scrollWidth <= el.clientWidth) return
      ev.preventDefault()
      el.scrollLeft += ev.deltaY
    }
    el.addEventListener('wheel', wheel, { passive: false })
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure)
    observer?.observe(el)
    return () => {
      el.removeEventListener('wheel', wheel)
      observer?.disconnect()
    }
  }, [measure])

  // a key added or taken away changes what overflows
  React.useEffect(measure, [children, measure])

  // (by its own scroll, not scrollIntoView, which would scroll the boxes
  // around it too)
  React.useEffect(() => {
    const el = ref.current
    if(selectedId === null || !el) return
    const key = el.querySelector<HTMLElement>(`[data-pattern-id="${selectedId}"]`)
    if(!key) return
    const box = el.getBoundingClientRect()
    const at = key.getBoundingClientRect()
    if(at.left < box.left + FADE) el.scrollLeft -= box.left + FADE - at.left
    else if(at.right > box.right - FADE) el.scrollLeft += at.right - (box.right - FADE)
  }, [selectedId])

  const mask = `linear-gradient(to right, ${more.left ? 'transparent' : 'black'}, black ${FADE}px, `
    + `black calc(100% - ${FADE}px), ${more.right ? 'transparent' : 'black'})`

  return (
    <div
      ref={ref}
      className='min-w-0 flex [scrollbar-width:none] [&::-webkit-scrollbar]:hidden flex-row items-end gap-1.5 overflow-x-auto overflow-y-hidden pb-1'
      style={{ maskImage: mask, WebkitMaskImage: mask }}
      onScroll={measure}
    >
      {children}
    </div>
  )
}

// the ▾ by the keys: every pattern in a list, with a search at the top, to
// pick one that's scrolled out of sight (Enter picks the first found), or to
// drag into the playlist (the list gets out of the way as the drag starts).
// the selected one is ticked, and the playing one lit
export const PatternMenu = ({ patterns, selectedId, playingId, onPick }: {
  patterns: Pattern[]
  selectedId: number | null
  playingId: number | null
  onPick: (patternId: number) => void
}) => {
  const button = React.useRef<HTMLDivElement>(null)
  const list = React.useRef<HTMLDivElement>(null)
  // where it opens, in the window
  const [at, setAt] = React.useState<{ left: number, top: number } | null>(null)
  const [query, setQuery] = React.useState('')
  const close = React.useCallback(() => setAt(null), [])
  const found = searchPatterns(patterns, query)
  const pick = (patternId: number) => {
    close()
    onPick(patternId)
  }

  // a pattern dragged out of the list: close it, so the playlist can be seen
  React.useEffect(() => {
    if(!at) return undefined
    return onSampleDrag(({ type }) => { if(type === 'move') close() })
  }, [at, close])

  React.useEffect(() => {
    if(!at) return undefined
    const outside = (ev: Event) => {
      const target = ev.target as Node
      if(!list.current?.contains(target) && !button.current?.contains(target)) close()
    }
    const escape = (ev: KeyboardEvent) => { if(ev.key === 'Escape') close() }
    window.addEventListener('mousedown', outside, true)
    window.addEventListener('keydown', escape)
    window.addEventListener('blur', close)
    return () => {
      window.removeEventListener('mousedown', outside, true)
      window.removeEventListener('keydown', escape)
      window.removeEventListener('blur', close)
    }
  }, [at, close])

  return (
    <div ref={button} className='shrink-0 pb-1'>
      <AnalogButton
        size='sm'
        variant='text'
        latched={!!at}
        aria-label='All patterns'
        aria-haspopup='menu'
        aria-expanded={!!at}
        title='Every pattern, to pick one'
        onPress={() => {
          if(at) {
            close()
            return
          }
          const box = button.current?.getBoundingClientRect()
          setQuery('')
          if(box) setAt({ left: Math.max(4, Math.min(box.left, window.innerWidth - 228)), top: box.bottom + 4 })
        }}
      >
        ▾
      </AnalogButton>
      {at && createPortal(
        <div
          ref={list}
          role='menu'
          className='fixed z-50 w-56 max-h-[60vh] overflow-y-auto py-1 rounded-md bg-raised text-xs text-ink shadow-lg ring-1 ring-line-strong select-none'
          style={at}
        >
          <div className='px-2 pt-1 pb-1.5 mb-1 border-b border-line'>
            <input
              type='search'
              // eslint-disable-next-line jsx-a11y/no-autofocus
              autoFocus
              aria-label='Search patterns'
              placeholder='Search patterns'
              className='ft-readout w-full h-7 px-2 text-xs'
              value={query}
              onChange={e => setQuery(e.currentTarget.value)}
              onKeyDown={e => {
                if(e.key === 'Enter' && found.length) pick(found[0].id)
              }}
            />
          </div>
          {!found.length && <p className='px-3 py-1.5 text-ink-faint'>No patterns match</p>}
          {found.map(pattern => (
            <button
              key={pattern.id}
              type='button'
              role='menuitemradio'
              aria-checked={pattern.id === selectedId}
              aria-label={patternLabel(pattern)}
              title='Click to show it, or drag it into the playlist'
              className='w-full flex flex-row items-center gap-2 px-3 py-1.5 text-left hover:bg-accent-tint cursor-grab'
              onMouseDown={ev => beginSampleDrag({ kind: 'pattern', patternId: pattern.id, label: patternLabel(pattern) }, ev)}
              onClick={() => pick(pattern.id)}
            >
              <span className='w-3 shrink-0 text-accent-strong'>{pattern.id === selectedId ? '✓' : ''}</span>
              <span className='flex-1 min-w-0 truncate'>{patternLabel(pattern)}</span>
              {pattern.id === playingId && <Led color='lime' state='on' />}
            </button>
          ))}
        </div>,
        document.body,
      )}
    </div>
  )
}
