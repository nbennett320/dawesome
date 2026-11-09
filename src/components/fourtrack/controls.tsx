import React from 'react'

// skeuomorphic controls modelled on a cassette four-track (see fourtrack.css)

const cx = (...c: Array<string | false | null | undefined>) => c.filter(Boolean).join(' ')
const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v))

// a value that's either controlled (value given) or kept here
function useControllable<T>(value: T | undefined, defaultValue: T, onChange?: (v: T) => void) {
  const [inner, setInner] = React.useState(defaultValue)
  const controlled = value !== undefined
  const current = controlled ? (value as T) : inner
  const set = React.useCallback(
    (v: T) => {
      if(!controlled) setInner(v)
      onChange?.(v)
    },
    [controlled, onChange],
  )
  return [current, set] as const
}

/* ------------------------------------------------------------------ */
/* AnalogButton                                                        */
/* ------------------------------------------------------------------ */

export type AnalogButtonSize = 'xs' | 'sm' | 'md' | 'lg'

export interface AnalogButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'onClick'> {
  // an icon (or short text), centred on the cap
  children?: React.ReactNode
  size?: AnalogButtonSize
  // held down like a latching key. leave it out for momentary keys
  latched?: boolean
  // a short text cap (e.g. MUTE) instead of an icon
  variant?: 'icon' | 'text'
  onPress?: (e: React.MouseEvent<HTMLButtonElement>) => void
  className?: string
  type?: 'button' | 'submit' | 'reset'
}

export const AnalogButton = React.forwardRef<HTMLButtonElement, AnalogButtonProps>(
  ({ children, size = 'md', latched, variant = 'icon', onPress, className, type = 'button', ...rest }, ref) => (
    <button
      ref={ref}
      // eslint-disable-next-line react/button-has-type
      type={type}
      aria-pressed={latched === undefined ? undefined : latched}
      className={cx(
        'ft-btn',
        size !== 'md' && `ft-btn--${size}`,
        variant === 'text' && 'ft-btn--key',
        className,
      )}
      onClick={onPress}
      {...rest}
    >
      {variant === 'text' ? children : <span className='ft-btn__icon'>{children}</span>}
    </button>
  ),
)
AnalogButton.displayName = 'AnalogButton'

/* ------------------------------------------------------------------ */
/* Led                                                                 */
/* ------------------------------------------------------------------ */

export type LedState = 'off' | 'on' | 'dim' | 'blink'
export type LedColor = 'red' | 'amber' | 'lime' | 'blue'

export interface LedProps extends React.HTMLAttributes<HTMLSpanElement> {
  color?: LedColor
  state?: LedState
}

export const Led = ({ color = 'red', state = 'off', className, ...rest }: LedProps) => (
  <span
    aria-hidden
    data-state={state}
    className={cx('ft-led', `ft-led--${color}`, className)}
    {...rest}
  />
)

/* ------------------------------------------------------------------ */
/* Knob                                                                */
/* ------------------------------------------------------------------ */

export interface KnobProps {
  value?: number
  defaultValue?: number
  onChange?: (value: number) => void
  min?: number
  max?: number
  step?: number
  // what double-clicking or the "0" key goes back to. the midpoint by default
  center?: number
  // the whole sweep in degrees
  sweep?: number
  // pixels of vertical drag from min to max
  dragRange?: number
  size?: number
  label: string
  // the value for screen readers, e.g. "L 40"
  formatValue?: (value: number) => string
  disabled?: boolean
  className?: string
  title?: string
}

// drag up and down (shift for fine moves), arrow keys (shift for big steps),
// Home/End, or double-click to centre
export const Knob = ({
  value,
  defaultValue,
  onChange,
  min = -1,
  max = 1,
  step = 0.01,
  center = (min + max) / 2,
  sweep = 270,
  dragRange = 180,
  size = 64,
  label,
  formatValue,
  disabled,
  className,
  title,
}: KnobProps) => {
  const [v, setV] = useControllable(value, defaultValue ?? center, onChange)
  const drag = React.useRef<{ y: number, v: number } | null>(null)

  const commit = (next: number) => {
    const snapped = Math.round(clamp(next, min, max) / step) * step
    setV(Number(snapped.toFixed(6)))
  }

  const t = (v - min) / (max - min)
  const angle = (t - 0.5) * sweep
  const coarse = (max - min) / 10
  const fine = step * 5

  const endDrag = () => { drag.current = null }

  return (
    <div
      role='slider'
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={v}
      aria-valuetext={formatValue?.(v)}
      aria-disabled={disabled || undefined}
      title={title}
      className={cx('ft-knob', className)}
      style={{ width: size, height: size, opacity: disabled ? 0.5 : undefined }}
      onPointerDown={e => {
        if(disabled || e.button !== 0) return
        e.currentTarget.setPointerCapture(e.pointerId)
        e.currentTarget.focus()
        drag.current = { y: e.clientY, v }
      }}
      onPointerMove={e => {
        if(!drag.current) return
        const scale = e.shiftKey ? 0.25 : 1
        const next = drag.current.v + ((drag.current.y - e.clientY) / dragRange) * (max - min) * scale
        commit(next)
        // fine moves are relative to here, so switching shift doesn't jump
        if(e.shiftKey) drag.current = { y: e.clientY, v: next }
      }}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onDoubleClick={() => { if(!disabled) commit(center) }}
      onKeyDown={e => {
        if(disabled) return
        const s = e.shiftKey ? coarse : fine
        const next = ({
          ArrowUp: v + s,
          ArrowRight: v + s,
          ArrowDown: v - s,
          ArrowLeft: v - s,
          PageUp: v + coarse,
          PageDown: v - coarse,
          Home: min,
          End: max,
          0: center,
        } as Record<string, number | undefined>)[e.key]
        if(next !== undefined) {
          e.preventDefault()
          commit(next)
        }
      }}
    >
      <span className='ft-knob__rot' style={{ transform: `rotate(${angle}deg)` }}>
        <span className='ft-knob__cap' />
        <span className='ft-knob__ptr' />
      </span>
    </div>
  )
}

/* ------------------------------------------------------------------ */
/* MasterFader                                                         */
/* ------------------------------------------------------------------ */

export interface MasterFaderProps {
  value?: number
  defaultValue?: number
  onChange?: (value: number) => void
  min?: number
  max?: number
  step?: number
  // the track's height in px
  height?: number
  // scale labels. every fifth of the range by default
  ticks?: number[]
  label?: string
  showReadout?: boolean
  className?: string
}

const CAP_H = 52

// a linear fader over a native range input (the mixer's faders use their own
// dB scale, see views/mixer/Fader)
export const MasterFader = ({
  value,
  defaultValue = 7.5,
  onChange,
  min = 0,
  max = 10,
  step = 0.1,
  height = 280,
  ticks,
  label = 'Master level',
  showReadout = true,
  className,
}: MasterFaderProps) => {
  const [v, setV] = useControllable(value, defaultValue, onChange)
  const travel = height - CAP_H
  const pos = (n: number) => (1 - (n - min) / (max - min)) * travel
  const scale = ticks ?? Array.from({ length: 6 }, (_, i) => max - i * ((max - min) / 5))

  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10 }}>
      <div className={cx('ft-fader', className)} style={{ height }}>
        {scale.map(t => (
          <React.Fragment key={t}>
            <span className='ft-fader__tick' style={{ top: pos(t) + CAP_H / 2 - 7 }}>{+t.toFixed(1)}</span>
            <span className='ft-fader__line' style={{ top: pos(t) + CAP_H / 2 }} />
          </React.Fragment>
        ))}
        <div className='ft-fader__slot' />
        <input
          className='ft-fader__input'
          type='range'
          min={min}
          max={max}
          step={step}
          value={v}
          aria-label={label}
          aria-orientation='vertical'
          onChange={e => setV(parseFloat(e.target.value))}
        />
        <div className='ft-fader__cap' style={{ top: pos(v) }} />
      </div>
      {showReadout && <span className='ft-readout'>{v.toFixed(1)}</span>}
    </div>
  )
}

/* ------------------------------------------------------------------ */
/* MuteSolo                                                            */
/* ------------------------------------------------------------------ */

export interface MuteSoloProps {
  // the track's name, for labels, e.g. "Track 1"
  track: string
  muted: boolean
  soloed: boolean
  // silenced because another track is soloed: the mute LED glows dimly
  implied?: boolean
  onMutedChange: (muted: boolean) => void
  onSoloedChange: (soloed: boolean) => void
  size?: AnalogButtonSize
  // side by side, or stacked
  direction?: 'row' | 'column'
}

const muteLed = (muted: boolean, implied?: boolean): LedState => {
  if(muted) return 'on'
  return implied ? 'dim' : 'off'
}

// mute (amber) and solo (lime) keys, each with an LED
export const MuteSolo = ({
  track,
  muted,
  soloed,
  implied,
  onMutedChange,
  onSoloedChange,
  size = 'md',
  direction = 'column',
}: MuteSoloProps) => (
  <div className={cx('ft-mutesolo', direction === 'row' && 'ft-mutesolo--row')}>
    <div className='ft-mutesolo__key'>
      <Led color='amber' state={muteLed(muted, implied)} />
      <AnalogButton
        variant='text'
        size={size}
        latched={muted}
        aria-label={`Mute ${track}`}
        title={implied ? 'Mute (silenced by another track\'s solo)' : 'Mute'}
        onPress={() => onMutedChange(!muted)}
      >
        M
      </AnalogButton>
    </div>
    <div className='ft-mutesolo__key'>
      <Led color='lime' state={soloed ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size={size}
        latched={soloed}
        aria-label={`Solo ${track}`}
        title='Solo: only hear soloed tracks'
        onPress={() => onSoloedChange(!soloed)}
      >
        S
      </AnalogButton>
    </div>
  </div>
)

/* ------------------------------------------------------------------ */
/* LevelMeter                                                          */
/* ------------------------------------------------------------------ */

export interface LevelMeterProps {
  // 0 to 1
  level: number
  segments?: Array<{ label: string, threshold: number, color: LedColor }>
}

const DEFAULT_SEGMENTS: NonNullable<LevelMeterProps['segments']> = [
  { label: '-10', threshold: 0.28, color: 'lime' },
  { label: '-5', threshold: 0.5, color: 'amber' },
  { label: '0', threshold: 0.72, color: 'red' },
  { label: '+6', threshold: 0.9, color: 'red' },
]

// a row of LEDs that light as the level rises
export const LevelMeter = ({ level, segments = DEFAULT_SEGMENTS }: LevelMeterProps) => (
  <div className='ft-meter' role='meter' aria-label='Level' aria-valuemin={0} aria-valuemax={1} aria-valuenow={level}>
    {segments.map(s => (
      <div key={s.label} className='ft-meter__cell'>
        <Led color={s.color} state={level > s.threshold ? 'on' : 'off'} />
        <span className='ft-label'>{s.label}</span>
      </div>
    ))}
  </div>
)

/* ------------------------------------------------------------------ */
/* TapeCounter                                                         */
/* ------------------------------------------------------------------ */

export interface TapeCounterProps {
  value: number
  digits?: number
  label?: string
}

// mechanical counter wheels
export const TapeCounter = ({ value, digits = 3, label = 'Counter' }: TapeCounterProps) => {
  const mod = 10 ** digits
  const text = String(((Math.floor(value) % mod) + mod) % mod).padStart(digits, '0')
  return (
    <div className='ft-counter' role='img' aria-label={`${label} ${text}`}>
      {text.split('').map((d, i) => (
        // the wheels are fixed in place, so their position is their identity
        // eslint-disable-next-line react/no-array-index-key
        <span key={i} className='ft-counter__digit'>{d}</span>
      ))}
    </div>
  )
}
