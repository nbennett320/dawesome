import React from 'react'
import { cx } from './util'

// how long a wheel takes to roll to its next digit, in ms
export const ROLL_MS = 180

// where a wheel goes when its digit changes, as positions on its strip
// (0-9, then 0 again at 10, so 9 → 0 keeps rolling the same way)
// - roll: animate from `from` to `to`
// - jump: go straight there
export type WheelMove =
  | { kind: 'roll', from: number, to: number }
  | { kind: 'jump', to: number }

// one step forward rolls up, one step back rolls down; anything else (a
// bigger jump, or changes coming faster than a wheel can roll) jumps
export const wheelMove = (previous: number | null, next: number, rolling: boolean): WheelMove => {
  if(previous === null || !rolling) return { kind: 'jump', to: next }

  // forward: 9 → 0 rolls on to the 0 at the end of the strip
  if(next === (previous + 1) % 10) return { kind: 'roll', from: previous, to: next === 0 ? 10 : next }
  // back: 0 → 9 starts from the 0 at the end of the strip
  if(next === (previous + 9) % 10) return { kind: 'roll', from: previous === 0 ? 10 : previous, to: next }

  return { kind: 'jump', to: next }
}

const STRIP = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0]

const offset = (position: number) => `translateY(calc(${-position} * var(--ft-roll-h)))`

// a counter wheel for one digit, which rolls to each new digit
const Wheel = ({ digit }: { digit: number }) => {
  const strip = React.useRef<HTMLSpanElement>(null)
  const previous = React.useRef<number | null>(null)
  const lastChange = React.useRef(0)

  React.useLayoutEffect(() => {
    const el = strip.current
    if(!el) return

    const now = performance.now()
    const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    // changes arriving faster than a roll (winding, tenths of a second) jump
    const rolling = !still && now - lastChange.current >= ROLL_MS
    lastChange.current = now

    const move = wheelMove(previous.current, digit, rolling)
    previous.current = digit

    if(move.kind === 'jump') {
      el.style.transition = 'none'
      el.style.transform = offset(move.to)
      return
    }

    // start where the roll starts, then roll
    el.style.transition = 'none'
    el.style.transform = offset(move.from)
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    el.offsetHeight // apply the start before animating
    el.style.transition = `transform ${ROLL_MS}ms cubic-bezier(.3, .7, .4, 1)`
    el.style.transform = offset(move.to)
  }, [digit])

  // a roll onto the strip's last 0 settles back on its first
  const settle = () => {
    const el = strip.current
    if(!el || previous.current !== 0) return
    el.style.transition = 'none'
    el.style.transform = offset(0)
  }

  return (
    <span className='ft-roll__wheel'>
      <span ref={strip} className='ft-roll__strip' onTransitionEnd={settle}>
        {STRIP.map((d, i) => (
          // the strip never changes, so its position is its identity
          // eslint-disable-next-line react/no-array-index-key
          <span key={i} className='ft-roll__digit'>{d}</span>
        ))}
      </span>
    </span>
  )
}

export interface RollingCounterProps {
  // what to show, e.g. '014' or '1:05.3'. digits get wheels, anything else
  // (':', '.') is printed between them
  value: string
  // read out with the value
  label: string
  className?: string
}

// mechanical counter wheels that roll from one number to the next. a carry
// (09 → 10) rolls every wheel it changes together
export const RollingCounter = ({ value, label, className }: RollingCounterProps) => {
  const chars = value.split('')

  return (
    <span className={cx('ft-roll', className)} role='img' aria-label={`${label} ${value}`}>
      {chars.map((c, i) => {
        // keyed from the right, so the wheels keep their place when the
        // value gets longer (9:59 → 10:00)
        const key = chars.length - i
        return /\d/.test(c)
          ? <Wheel key={key} digit={Number(c)} />
          : <span key={key} className='ft-roll__sep' aria-hidden>{c}</span>
      })}
    </span>
  )
}
