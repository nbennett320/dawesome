import React from 'react'

export interface ClipMenuItem {
  label: string
  // the keyboard shortcut that does the same, shown on the right
  shortcut?: string
  onSelect: () => void
  danger?: boolean
}

interface Props {
  // where the menu opens, relative to the playlist's box
  x: number
  y: number
  items: ClipMenuItem[]
  onClose: () => void
}

// a clip's right-click menu. closes on a choice, a click anywhere else,
// Escape, scrolling, or the window losing focus
const ClipMenu = ({ x, y, items, onClose }: Props) => {
  const ref = React.useRef<HTMLDivElement>(null)
  // kept inside the playlist's box, once it's been measured
  const [shift, setShift] = React.useState({ x: 0, y: 0 })

  React.useLayoutEffect(() => {
    const el = ref.current
    const box = el?.offsetParent?.getBoundingClientRect()
    if(!el || !box) return
    const { width, height } = el.getBoundingClientRect()
    setShift({
      x: Math.min(0, box.width - (x + width) - 4),
      y: Math.min(0, box.height - (y + height) - 4),
    })
  }, [x, y])

  React.useEffect(() => {
    const outside = (ev: Event) => {
      if(!ref.current?.contains(ev.target as Node)) onClose()
    }
    const escape = (ev: KeyboardEvent) => {
      if(ev.key === 'Escape') onClose()
    }

    window.addEventListener('mousedown', outside, true)
    window.addEventListener('wheel', onClose, true)
    window.addEventListener('keydown', escape)
    window.addEventListener('blur', onClose)
    return () => {
      window.removeEventListener('mousedown', outside, true)
      window.removeEventListener('wheel', onClose, true)
      window.removeEventListener('keydown', escape)
      window.removeEventListener('blur', onClose)
    }
  }, [onClose])

  return (
    <div
      ref={ref}
      role='menu'
      tabIndex={-1}
      className='absolute z-50 min-w-[11rem] py-1 rounded-md bg-white text-xs text-slate-700 shadow-lg ring-1 ring-slate-300 select-none'
      style={{ left: x + shift.x, top: y + shift.y }}
      onContextMenu={e => e.preventDefault()}
    >
      {items.map(item => (
        <button
          key={item.label}
          type='button'
          role='menuitem'
          className={`w-full flex flex-row items-center justify-between gap-4 px-3 py-1.5 text-left hover:bg-sky-50 ${
            item.danger ? 'text-red-600 hover:bg-red-50' : ''
          }`}
          onClick={() => {
            onClose()
            item.onSelect()
          }}
        >
          <span>{item.label}</span>
          {item.shortcut && <span className='text-slate-400 font-mono'>{item.shortcut}</span>}
        </button>
      ))}
    </div>
  )
}

export default ClipMenu
