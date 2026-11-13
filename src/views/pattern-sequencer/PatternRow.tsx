import React from 'react'
import {
  Note, Row, greyedSteps, rowName, stepView, stepsPerBar,
} from '../../types/sequencer'
import InstrumentControls from './InstrumentControls'
import NotePreview from './NotePreview'
import StepKeys, { Paint } from './StepKeys'
import { barX, gridWidth } from './stepGrid'

interface Props {
  row: Row
  // its notes in the pattern shown, and the pattern's length in steps
  notes: Note[]
  length: number
  beatsPerBar: number
  // the bars in view (see useVisibleBars)
  fromBar: number
  toBar: number
  paint: React.MutableRefObject<Paint | null>
  onStep: (rowId: number, step: number, on: boolean) => void
  // opens the Piano Roll on its notes
  onOpenNotes: (rowId: number) => void
  // dragged by its grip now
  dragging: boolean
  // the name of a sample or instrument from the sidebar dragged over it,
  // which would replace its own
  replacingWith: string | null
  // a row dragged would go just above or below it
  dropLine: 'before' | 'after' | null
  onGripDown: (rowId: number, ev: React.MouseEvent) => void
}

const NONE_LIT: ReadonlySet<number> = new Set()

// one row: its controls, then a key for each of the pattern's steps, or a
// picture of its notes when they aren't plain steps (see stepView), and a
// bar of greyed keys past the end, to make the pattern longer. memoised: it
// renders again only when it or its own notes, the pattern's length, or the
// bars in view change (and then only the parts of it they change)
const PatternRow = ({
  row, notes, length, beatsPerBar, fromBar, toBar, paint, onStep, onOpenNotes, dragging, replacingWith, dropLine,
  onGripDown,
}: Props) => {
  const name = rowName(row)
  const bar = stepsPerBar(beatsPerBar)
  const greyed = greyedSteps(length, bar)
  // the steps lit, worked out again only when the row or its notes change
  const lit = React.useMemo(() => {
    const steps = stepView(row, notes, length)
    return steps && new Set(steps)
  }, [row, notes, length])
  const step = React.useCallback((s: number, on: boolean) => onStep(row.id, s, on), [onStep, row.id])
  // (the Piano Roll edits synth rows' notes: a sample row's are only shown)
  const open = row.instrument.kind === 'synth' ? () => onOpenNotes(row.id) : undefined
  const keys = { name, length, beatsPerBar, fromBar, toBar, paint, onStep: step }

  return (
    <div
      data-row-id={row.id}
      className={`relative flex flex-row items-center gap-2 rounded-md ${dragging ? 'opacity-50' : ''} ${
        replacingWith ? 'ring-2 ring-accent-focus ring-offset-2 ring-offset-panel' : ''
      }`}
    >
      {dropLine && (
        <div
          aria-hidden='true'
          className={`absolute left-0 right-0 h-0.5 rounded-full bg-accent pointer-events-none z-10 ${
            dropLine === 'before' ? '-top-1' : '-bottom-1'
          }`}
        />
      )}
      <InstrumentControls row={row} onGripDown={onGripDown} />
      <div
        className={`seq-steps ${row.muted ? 'is-muted' : ''}`}
        style={{ width: gridWidth(length + greyed, beatsPerBar) }}
      >
        {lit
          ? <StepKeys {...keys} first={0} last={length + greyed} lit={lit} />
          : (
            <>
              <NotePreview
                name={name}
                notes={notes}
                length={length}
                beatsPerBar={beatsPerBar}
                fromBar={fromBar}
                toBar={toBar}
                muted={row.muted || (row.instrument.kind === 'synth' && row.instrument.missing)}
                onOpen={open}
              />
              <StepKeys {...keys} first={length} last={length + greyed} lit={NONE_LIT} />
            </>
          )}
        {replacingWith && (
          // (over the bars in view, so what it says is in view too)
          <div
            aria-hidden='true'
            className='absolute top-0 bottom-0 flex items-center justify-center rounded-sm bg-accent-tint/90 text-xs text-accent-ink pointer-events-none'
            style={{
              left: barX(fromBar, beatsPerBar),
              width: gridWidth(Math.min(toBar * bar, length + greyed), beatsPerBar) - barX(fromBar, beatsPerBar),
            }}
          >
            Replace {row.instrument.kind === 'sample' ? 'sample' : 'instrument'} with {replacingWith}
          </div>
        )}
      </div>
    </div>
  )
}

export default React.memo(PatternRow)
