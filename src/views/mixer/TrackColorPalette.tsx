// a track's colors as LEDs: the tray of lit LEDs a color is picked from,
// and the CSS for an LED in a track's color. the colors are the theme's
// `track` palette, so they're painted inline (with the painter) and
// repainted when the theme changes
import React from 'react'
import { painter, useThemeChange } from '../../theme'
import { Led } from '../../components/fourtrack'
import { TRACK_COLORS, trackColorKey } from '../../types/mixer'

const trackPaint = painter('track')

// a track's color as CSS
export const trackColorCss = (color: number): string => trackPaint.css(trackColorKey(color))

// an LED's colors from any CSS color: lit, its glow, and unlit. unlit is
// stronger than the fixed LEDs' tint, so a quiet track's color still shows
export const ledTint = (css: string) => ({
  '--led': css,
  '--led-glow': `color-mix(in srgb, ${css} 45%, transparent)`,
  '--led-off': `color-mix(in srgb, ${css} 45%, var(--ft-panel))`,
} as React.CSSProperties)

// the palette as a tray of lit LEDs. closes on a pick, Escape, or a press
// outside it
export const TrackColorPalette = ({ label, color, onChange, onClose, className, style }: {
  label: string
  color: number
  onChange: (color: number) => void
  onClose: () => void
  className?: string
  style?: React.CSSProperties
}) => {
  useThemeChange()
  const ref = React.useRef<HTMLDivElement>(null)
  const current = trackColorKey(color)

  React.useEffect(() => {
    const onDown = (e: PointerEvent) => {
      const target = e.target as Node
      if(!ref.current?.contains(target)) onClose()
    }
    const onKey = (e: KeyboardEvent) => {
      if(e.key === 'Escape') onClose()
    }
    window.addEventListener('pointerdown', onDown)
    window.addEventListener('keydown', onKey)
    return () => {
      window.removeEventListener('pointerdown', onDown)
      window.removeEventListener('keydown', onKey)
    }
  }, [onClose])

  // focus the current color, so the arrow-less grid can be tabbed from there
  React.useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus()
  }, [])

  return (
    <div
      ref={ref}
      role='menu'
      aria-label={`${label} color`}
      className={`ft-color-tray ${className ?? ''}`}
      style={style}
    >
      {TRACK_COLORS.map((key, i) => (
        <button
          key={key}
          type='button'
          role='menuitemradio'
          aria-checked={key === current}
          aria-label={key}
          title={key[0].toUpperCase() + key.slice(1)}
          className='ft-jewel'
          onClick={() => {
            onChange(i)
            onClose()
          }}
        >
          <Led state='on' style={ledTint(trackPaint.css(key))} />
        </button>
      ))}
    </div>
  )
}
