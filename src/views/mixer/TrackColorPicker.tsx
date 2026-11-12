// a track's color, shown as a small square beside its name. clicking it
// opens the palette to pick another from. the colors are the theme's `track`
// palette, so they're painted inline (with the painter) rather than as
// Tailwind classes, and repainted when the theme changes
import React from 'react'
import { painter, useThemeChange } from '../../theme'
import { TRACK_COLORS, trackColorKey } from '../../types/mixer'

const trackPaint = painter('track')

// a track's color as CSS, e.g. for its strip's --ft-control-accent
export const trackColorCss = (color: number): string => trackPaint.css(trackColorKey(color))

const TrackColorPicker = ({ label, color, onChange }: {
  // the track's name, for screen readers and tooltips
  label: string
  // its color (see TRACK_COLORS)
  color: number
  onChange: (color: number) => void
}) => {
  useThemeChange()
  const [open, setOpen] = React.useState(false)
  const ref = React.useRef<HTMLDivElement>(null)

  // a press anywhere else, or Escape, closes the palette
  React.useEffect(() => {
    if(!open) return undefined
    const onDown = (e: PointerEvent) => {
      if(!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    const onKey = (e: KeyboardEvent) => {
      if(e.key === 'Escape') setOpen(false)
    }
    window.addEventListener('pointerdown', onDown)
    window.addEventListener('keydown', onKey)
    return () => {
      window.removeEventListener('pointerdown', onDown)
      window.removeEventListener('keydown', onKey)
    }
  }, [open])

  const current = trackColorKey(color)

  return (
    <div ref={ref} className='relative flex-none flex items-center'>
      <button
        type='button'
        className='block w-2.5 h-2.5 rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-accent-focus'
        style={{ background: trackPaint.css(current) }}
        aria-label={`${label} color`}
        aria-haspopup='true'
        aria-expanded={open}
        title='Track color. Click to change it'
        onClick={() => setOpen(o => !o)}
      />
      {open && (
        <div
          role='menu'
          aria-label={`${label} color`}
          className='absolute left-0 top-full mt-1 z-20 grid grid-cols-4 gap-1 p-1.5 rounded bg-panel border border-line shadow'
        >
          {TRACK_COLORS.map((key, i) => (
            <button
              key={key}
              type='button'
              role='menuitemradio'
              aria-checked={key === current}
              aria-label={key}
              title={key[0].toUpperCase() + key.slice(1)}
              className={`w-4 h-4 rounded-sm ${key === current ? 'ring-2 ring-offset-1 ring-ink ring-offset-panel' : ''}`}
              style={{ background: trackPaint.css(key) }}
              onClick={() => {
                onChange(i)
                setOpen(false)
              }}
            />
          ))}
        </div>
      )}
    </div>
  )
}

export default TrackColorPicker
