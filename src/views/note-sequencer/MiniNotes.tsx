import React from 'react'
import { Note, noteBars, stepsPerBar } from '../../types/sequencer'
import { clamp } from '../../util/helpers'

// how wide the picture is: a few pixels a step, within limits, so a long
// pattern is squeezed rather than scrolled
const PX_PER_STEP = 6
const MIN_WIDTH = 192
const MAX_WIDTH = 640

interface Props {
  name: string
  notes: Note[]
  // the pattern's length, in steps
  length: number
  beatsPerBar: number
  muted: boolean
  // opens the Piano Roll on them. without it, the picture is only looked at
  onOpen?: () => void
}

// a small picture of a row's notes in a pattern, where steps would be: click
// it to edit them in the Piano Roll
const MiniNotes = ({ name, notes, length, beatsPerBar, muted, onOpen }: Props) => {
  const { rows, bars } = React.useMemo(() => noteBars(notes, length, 0, length), [notes, length])
  const width = clamp(length * PX_PER_STEP, MIN_WIDTH, MAX_WIDTH)
  const bar = stepsPerBar(beatsPerBar)
  const lines = Array.from({ length: Math.max(0, Math.ceil(length / bar) - 1) }, (_, i) => (i + 1) * bar)

  const picture = (
    <>
      <svg
        className='absolute inset-0 w-full h-full'
        viewBox={`0 0 ${Math.max(1, length)} ${Math.max(1, rows)}`}
        preserveAspectRatio='none'
        aria-hidden
      >
        {lines.map(step => (
          <line
            key={step}
            x1={step}
            x2={step}
            y1={0}
            y2={Math.max(1, rows)}
            className='stroke-line'
            vectorEffect='non-scaling-stroke'
          />
        ))}
        {bars.map(({ start, end, row }, i) => (
          <rect
            // eslint-disable-next-line react/no-array-index-key -- notes have no ids, and are only drawn
            key={i}
            x={start}
            y={row + 0.1}
            width={Math.max(0.3, end - start)}
            height={0.8}
            className={muted ? 'fill-accent-soft' : 'fill-accent'}
          />
        ))}
      </svg>
      {!bars.length && <span className='relative text-[10px] text-ink-faint'>No notes</span>}
    </>
  )
  const shape = 'relative shrink-0 h-11 rounded-md border border-line-strong bg-raised overflow-hidden'

  if(!onOpen) {
    return (
      <div
        role='img'
        className={`${shape} flex items-center justify-center`}
        style={{ width }}
        aria-label={`${name} notes`}
        title='Notes that aren’t plain steps, so they can’t be clicked here'
      >
        {picture}
      </div>
    )
  }

  return (
    <button
      type='button'
      className={`${shape} hover:border-ink-faint`}
      style={{ width }}
      aria-label={`${name} notes`}
      title={`${bars.length ? `${bars.length} note${bars.length === 1 ? '' : 's'}` : 'No notes yet'}. Click to edit them in the Piano Roll`}
      onClick={onOpen}
    >
      {picture}
    </button>
  )
}

export default React.memo(MiniNotes)
