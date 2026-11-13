import React from 'react'
import { Note, noteBars, stepsPerBar } from '../../types/sequencer'
import { barX, gridWidth, timeX } from './stepGrid'

interface Props {
  name: string
  notes: Note[]
  // the pattern's length, in steps
  length: number
  beatsPerBar: number
  // the bars in view (see useVisibleBars): only their notes are drawn
  fromBar: number
  toBar: number
  muted: boolean
  // opens the Piano Roll on them. without it, the picture is only looked at
  onOpen?: () => void
}

// a picture of a row's notes in a pattern, where its keys would be and as
// wide (so it lines up with the other rows' keys): shown when they aren't
// plain steps (see stepView). click it to edit them in the Piano Roll
const NotePreview = ({ name, notes, length, beatsPerBar, fromBar, toBar, muted, onOpen }: Props) => {
  // every note heard, as a bar on a row of its pitch (see noteBars)
  const { rows, bars } = React.useMemo(() => noteBars(notes, length, 0, length), [notes, length])
  const bar = stepsPerBar(beatsPerBar)
  const lastBar = Math.min(toBar, Math.ceil(length / bar))
  const left = barX(fromBar, beatsPerBar)
  const width = Math.max(0, gridWidth(Math.min(lastBar * bar, length), beatsPerBar) - left)

  // those in the bars in view
  const from = fromBar * bar
  const to = Math.min(lastBar * bar, length)
  const shown = bars.filter(note => note.start < to && note.end > from)

  const panels = []
  for(let b = fromBar; b < lastBar; b++) {
    const x = barX(b, beatsPerBar)
    panels.push(
      <div
        key={b}
        className='seq-preview-bar'
        style={{ left: x, width: gridWidth(Math.min(bar, length - b * bar), beatsPerBar) }}
      />,
    )
  }

  const picture = (
    <>
      {panels}
      {width > 0 && (
        <svg
          className='absolute top-0 h-full'
          style={{ left, width }}
          viewBox={`0 0 ${width} ${Math.max(1, rows)}`}
          preserveAspectRatio='none'
          aria-hidden
        >
          {shown.map((note, i) => {
            const x = timeX(note.start, beatsPerBar) - left
            return (
              <rect
                // eslint-disable-next-line react/no-array-index-key -- notes have no ids, and are only drawn
                key={i}
                x={x}
                y={note.row + 0.1}
                width={Math.max(2, timeX(note.end, beatsPerBar, true) - left - x)}
                height={0.8}
                className={muted ? 'fill-accent-soft' : 'fill-accent'}
              />
            )
          })}
        </svg>
      )}
    </>
  )
  const box = { width: gridWidth(length, beatsPerBar) }
  const said = bars.length ? `${bars.length} note${bars.length === 1 ? '' : 's'}` : 'No notes yet'

  if(!onOpen) {
    return (
      <div
        role='img'
        className='seq-preview'
        style={box}
        aria-label={`${name} notes`}
        title={`${said}. They aren’t plain steps, so they can’t be clicked here`}
      >
        {picture}
      </div>
    )
  }

  return (
    <button
      type='button'
      className='seq-preview is-open'
      style={box}
      aria-label={`${name} notes`}
      title={`${said}. Click to edit them in the Piano Roll`}
      onClick={onOpen}
    >
      {picture}
    </button>
  )
}

export default React.memo(NotePreview)
