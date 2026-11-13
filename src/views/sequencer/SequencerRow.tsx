import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import {
  previewSequencerRow,
  removeSequencerRow,
  renameSequencerRow,
  setSequencerRowMuted,
  setSequencerRowPan,
  setSequencerRowVolume,
} from '../../state/slices/sequencerSlice'
import {
  Pattern,
  STEPS_PER_BEAT,
  SequencerRow as Row,
  isLit,
} from '../../types/sequencer'
import { fileName } from '../../util/helpers'
import RowControls from './RowControls'

// a step being painted: pressing a step lights it (or clears it), and every
// step the mouse moves over while held is set the same way
export interface Paint {
  on: boolean
}

interface Props {
  row: Row
  pattern: Pattern
  beatsPerBar: number
  // the step playing now, highlighted, or null
  playingStep: number | null
  paint: React.MutableRefObject<Paint | null>
  onStep: (rowId: number, step: number, on: boolean) => void
  // dragged by its grip now
  dragging: boolean
  // the name of a sample from the sidebar being dragged over it, which
  // would replace its own
  replacingWith: string | null
  // a row dragged would go just above or below it
  dropLine: 'before' | 'after' | null
  onGripDown: (rowId: number, ev: React.MouseEvent) => void
}

// one sample: its name (click to hear it, right-click to rename), mute,
// volume, pan and remove, then a key for each of the pattern's steps
const SequencerRow = ({
  row, pattern, beatsPerBar, playingStep, paint, onStep, dragging, replacingWith, dropLine, onGripDown,
}: Props) => {
  const dispatch = useAppDispatch()
  const sample = fileName(row.path)
  const name = row.label ?? sample
  const stepsPerBar = Math.max(1, beatsPerBar) * STEPS_PER_BEAT

  const steps = Array.from({ length: pattern.length }, (_, step) => {
    const lit = isLit(pattern, row.id, step)
    const beat = Math.floor(step / STEPS_PER_BEAT)
    const firstOfBeat = step % STEPS_PER_BEAT === 0
    const firstOfBar = step % stepsPerBar === 0

    let tone = 'bg-raised'
    if(lit) tone = row.muted ? 'bg-accent-soft' : 'bg-accent'
    // every other beat a little darker, so the beats read at a glance
    else if(beat % 2 === 1) tone = 'bg-panel'

    let gap = ''
    if(step > 0 && firstOfBar) gap = 'ml-3'
    else if(step > 0 && firstOfBeat) gap = 'ml-1.5'

    return (
      <button
        key={step}
        type='button'
        aria-label={`${name}, step ${step + 1}`}
        aria-pressed={lit}
        title={`Step ${step + 1}`}
        className={[
          'shrink-0 w-5 h-7 rounded-sm border',
          lit ? 'border-accent-strong shadow-inner' : 'border-line-strong hover:border-ink-faint',
          step === playingStep ? 'ring-2 ring-led' : '',
          tone,
          gap,
        ].join(' ')}
        onMouseDown={ev => {
          if(ev.button !== 0) return
          ev.preventDefault()
          // eslint-disable-next-line no-param-reassign
          paint.current = { on: !lit }
          onStep(row.id, step, !lit)
        }}
        onMouseEnter={() => {
          const p = paint.current
          if(p && p.on !== lit) onStep(row.id, step, p.on)
        }}
        onKeyDown={ev => {
          // keyboard: enter presses the key (space plays and pauses, see App)
          if(ev.key === 'Enter') {
            ev.preventDefault()
            onStep(row.id, step, !lit)
          }
        }}
      />
    )
  })

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
      <RowControls
        onGripDown={ev => onGripDown(row.id, ev)}
        name={name}
        note={row.label ? sample : undefined}
        nameTitle={`${row.path}. Click to hear it, right-click to rename`}
        onName={() => dispatch(previewSequencerRow(row.id))}
        rename={{
          initial: row.label ?? '',
          placeholder: sample,
          onRename: label => dispatch(renameSequencerRow(row.id, label)),
        }}
        muted={row.muted}
        onMute={muted => dispatch(setSequencerRowMuted(row.id, muted))}
        volume={row.volume}
        onVolume={v => dispatch(setSequencerRowVolume(row.id, v))}
        pan={row.pan}
        onPan={p => dispatch(setSequencerRowPan(row.id, p))}
        removeTitle='Remove the row, and its steps from every pattern'
        onRemove={() => dispatch(removeSequencerRow(row.id))}
      />
      <div className='relative flex flex-row items-center'>
        {steps}
        {replacingWith && (
          <div
            aria-hidden='true'
            className='absolute inset-0 flex items-center justify-center rounded-sm bg-accent-tint/90 text-xs text-accent-ink pointer-events-none'
          >
            Replace sample with {replacingWith}
          </div>
        )}
      </div>
    </div>
  )
}

export default React.memo(SequencerRow)
