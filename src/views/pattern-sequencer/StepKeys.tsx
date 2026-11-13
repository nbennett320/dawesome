import React from 'react'
import { STEPS_PER_BEAT, stepsPerBar } from '../../types/sequencer'
import { beatX } from './stepGrid'

// a step being painted: pressing a step lights it (or clears it), and every
// step the mouse moves over while held is set the same way
export interface Paint {
  on: boolean
}

interface Keys {
  // the row's name, for reading the keys out
  name: string
  // the keys shown: steps `first` up to `last`. those from `length` on are
  // past the pattern's end, greyed (and never lit)
  first: number
  last: number
  length: number
  // the steps lit
  lit: ReadonlySet<number>
  beatsPerBar: number
  paint: React.MutableRefObject<Paint | null>
  onStep: (step: number, on: boolean) => void
}

type BarProps = Keys & { bar: number }

// whether a bar shows the same: the same keys, and its own steps lit as
// they were (another bar's step lit doesn't change it)
const sameBar = (a: BarProps, b: BarProps): boolean => {
  const { lit: aLit, ...aRest } = a
  const { lit: bLit, ...bRest } = b
  if((Object.keys(aRest) as Array<keyof typeof aRest>).some(key => aRest[key] !== bRest[key])) return false
  const steps = stepsPerBar(a.beatsPerBar)
  for(let step = a.bar * steps; step < (a.bar + 1) * steps; step++) {
    if(aLit.has(step) !== bLit.has(step)) return false
  }
  return true
}

// one bar's keys, a beat at a time, each beat placed where the grid has it
// and coloured by its place in the bar (the first three in colour, the rest
// plain). a key that isn't shown keeps its place empty. memoised, so a bar
// scrolled into view, or one whose step was clicked, renders, and the rest
// of the row doesn't
const StepBar = React.memo(({
  bar, name, first, last, length, lit, beatsPerBar, paint, onStep,
}: BarProps) => {
  const key = (step: number) => {
    if(step < first || step >= last) return <span key={step} className='seq-key-space' />
    const greyed = step >= length
    const on = !greyed && lit.has(step)
    return (
      <button
        key={step}
        type='button'
        aria-label={`${name}, step ${step + 1}${greyed ? ', past the end' : ''}`}
        aria-pressed={on}
        title={greyed ? `Step ${step + 1}. Click to make the pattern this long` : `Step ${step + 1}`}
        className={`seq-key ${on ? 'is-lit' : ''} ${greyed ? 'is-greyed' : ''}`}
        onMouseDown={ev => {
          if(ev.button !== 0) return
          ev.preventDefault()
          // eslint-disable-next-line no-param-reassign
          paint.current = { on: !on }
          onStep(step, !on)
        }}
        onMouseEnter={() => {
          const p = paint.current
          if(p && p.on !== on) onStep(step, p.on)
        }}
        onKeyDown={ev => {
          // keyboard: enter presses the key (space plays and pauses, see App)
          if(ev.key === 'Enter') {
            ev.preventDefault()
            onStep(step, !on)
          }
        }}
      >
        <span className='seq-led' />
      </button>
    )
  }

  const beats: React.ReactNode[] = []
  const fromBeat = Math.max(bar * beatsPerBar, Math.floor(first / STEPS_PER_BEAT))
  const toBeat = Math.min((bar + 1) * beatsPerBar, Math.ceil(last / STEPS_PER_BEAT))
  for(let beat = fromBeat; beat < toBeat; beat++) {
    const inBar = beat % beatsPerBar
    const start = beat * STEPS_PER_BEAT
    beats.push(
      <div
        key={beat}
        className={`seq-beat is-beat-${Math.min(inBar, 3)} ${inBar % 2 ? 'is-alt' : ''} ${start >= length ? 'is-greyed' : ''}`}
        style={{ left: beatX(beat, beatsPerBar) }}
      >
        <div className='seq-keys'>
          {Array.from({ length: STEPS_PER_BEAT }, (_, i) => key(start + i))}
        </div>
        <div className='seq-strip' />
      </div>,
    )
  }
  return <>{beats}</>
}, sameBar)
StepBar.displayName = 'StepBar'

// a row's keys in the bars in view (`fromBar` up to `toBar`: see
// useVisibleBars)
const StepKeys = ({ fromBar, toBar, ...keys }: Keys & { fromBar: number, toBar: number }) => {
  const bar = stepsPerBar(keys.beatsPerBar)
  const bars: React.ReactNode[] = []
  for(let b = Math.max(fromBar, Math.floor(keys.first / bar)); b < Math.min(toBar, Math.ceil(keys.last / bar)); b++) {
    bars.push(<StepBar key={b} bar={b} {...keys} />)
  }
  return <>{bars}</>
}

export default StepKeys
