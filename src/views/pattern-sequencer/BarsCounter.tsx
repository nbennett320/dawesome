import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import { setPatternBars } from '../../state/slices/sequencerSlice'
import { MAX_PATTERN_BARS, Pattern, barsReached, patternBars } from '../../types/sequencer'
import { AnalogButton, RollingCounter } from '../../components/fourtrack'

// whether a typed length is one a pattern can be
const validBars = (bars: number) => Number.isInteger(bars) && bars >= 1 && bars <= MAX_PATTERN_BARS

// a pattern's length in bars, on counter wheels like the top bar's, with
// ▲ (a bar longer) and ▼ (a bar shorter) beside them. click the wheels to
// type a number, taken up on Enter or when the field is left. each is a
// length the pattern is never trimmed below. notes past a shortened end are
// kept, but don't play
const BarsCounter = ({ pattern, beatsPerBar }: { pattern: Pattern, beatsPerBar: number }) => {
  const dispatch = useAppDispatch()
  const bars = barsReached(pattern, beatsPerBar)
  // (a length that isn't whole bars, from before the time signature changed,
  // is made whole by any number)
  const whole = patternBars(pattern, beatsPerBar) !== null
  const [typing, setTyping] = React.useState(false)
  // (the field is left once: Enter or Escape, then the blur of it going)
  const left = React.useRef(false)

  const set = (to: number) => {
    if(validBars(to) && (to !== bars || !whole)) dispatch(setPatternBars(pattern.id, to))
  }
  const leave = (typed: string | null) => {
    if(left.current) return
    left.current = true
    if(typed !== null) set(Number(typed))
    setTyping(false)
  }

  return (
    <div className='flex flex-row items-center gap-1.5'>
      {typing ? (
        <input
          type='number'
          // eslint-disable-next-line jsx-a11y/no-autofocus
          autoFocus
          min={1}
          max={MAX_PATTERN_BARS}
          step={1}
          className='ft-readout text-sm py-0.5 w-14 h-7'
          aria-label='Bars'
          defaultValue={bars}
          onFocus={e => e.currentTarget.select()}
          onBlur={e => leave(e.currentTarget.value)}
          onKeyDown={e => {
            if(e.key === 'Enter') leave(e.currentTarget.value)
            if(e.key === 'Escape') leave(null)
          }}
        />
      ) : (
        <button
          type='button'
          className='rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent-focus'
          title={`How many bars long the pattern is, 1 to ${MAX_PATTERN_BARS}. Click to type`}
          onClick={() => {
            left.current = false
            setTyping(true)
          }}
        >
          <RollingCounter value={String(bars).padStart(3, '0')} label='Bars' />
        </button>
      )}
      <div className='flex flex-row gap-1'>
        <AnalogButton
          variant='text'
          size='xs'
          aria-label='A bar longer'
          title='A bar longer'
          disabled={bars >= MAX_PATTERN_BARS}
          onPress={() => set(bars + 1)}
        >
          ▲
        </AnalogButton>
        <AnalogButton
          variant='text'
          size='xs'
          aria-label='A bar shorter'
          title='A bar shorter. Notes past the end are kept, but don’t play'
          disabled={bars <= 1}
          onPress={() => set(bars - 1)}
        >
          ▼
        </AnalogButton>
      </div>
    </div>
  )
}

export default BarsCounter
