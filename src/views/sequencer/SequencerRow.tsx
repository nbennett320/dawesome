import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  previewRow,
  removeRow,
  renameRow,
  setRowMuted,
  setRowPan,
  setRowVolume,
} from '../../state/slices/sequencerSlice'
import {
  Note,
  Row,
  STEPS_PER_BEAT,
  SampleInstrument,
  stepView,
} from '../../types/sequencer'
import { fileName } from '../../util/helpers'
import MiniNotes from '../note-sequencer/MiniNotes'
import RowControls from './RowControls'
import './steps.scss'

// a step being painted: pressing a step lights it (or clears it), and every
// step the mouse moves over while held is set the same way
export interface Paint {
  on: boolean
}

interface Props {
  row: Row<SampleInstrument>
  // its notes in the pattern shown
  notes: Note[]
  // the pattern's length, in steps
  length: number
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

// one sample row: its name (click to hear it, right-click to rename), mute,
// volume, pan and remove, then a key for each of the pattern's steps. notes
// that aren't plain steps are shown as a picture instead (see stepView)
const SequencerRow = ({
  row, notes, length, beatsPerBar, playingStep, paint, onStep, dragging, replacingWith, dropLine, onGripDown,
}: Props) => {
  const dispatch = useAppDispatch()
  const { path } = row.instrument
  const sample = fileName(path)
  const name = row.label ?? sample
  // how the step keys look (a preference, see steps.scss)
  const keys = useAppSelector(state => state.appearance.preferences.stepKeys)
  const bar = Math.max(1, beatsPerBar)
  // the steps lit, worked out again only when the row or its notes change
  const litSteps = React.useMemo(() => {
    const steps = stepView(row, notes, length)
    return steps && new Set(steps)
  }, [row, notes, length])

  const key = (step: number) => {
    const lit = !!litSteps?.has(step)
    return (
      <button
        key={step}
        type='button'
        aria-label={`${name}, step ${step + 1}`}
        aria-pressed={lit}
        title={`Step ${step + 1}`}
        className={`seq-key ${lit ? 'is-lit' : ''} ${step === playingStep ? 'is-now' : ''}`}
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
      >
        <span className='seq-led' />
      </button>
    )
  }

  // the keys in beats of four, each beat coloured by its place in the bar
  // (the first three in colour, the rest plain), with a wider gap between bars
  const beats = () => Array.from({ length: Math.ceil(length / STEPS_PER_BEAT) }, (_, beat) => {
    const first = beat * STEPS_PER_BEAT
    const inBar = beat % bar
    let gap = ''
    if(beat > 0 && inBar === 0) gap = 'ml-3'
    else if(beat > 0) gap = 'ml-1.5'

    return (
      <div key={beat} className={`seq-beat is-beat-${Math.min(inBar, 3)} ${inBar % 2 ? 'is-alt' : ''} ${gap}`}>
        <div className='seq-keys'>
          {Array.from({ length: Math.min(STEPS_PER_BEAT, length - first) }, (_, i) => key(first + i))}
        </div>
        <div className='seq-strip' />
      </div>
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
        nameTitle={`${path}. Click to hear it, right-click to rename`}
        onName={() => dispatch(previewRow(row.id))}
        rename={{
          initial: row.label ?? '',
          placeholder: sample,
          onRename: label => dispatch(renameRow(row.id, label)),
        }}
        muted={row.muted}
        onMute={muted => dispatch(setRowMuted(row.id, muted))}
        volume={row.volume}
        onVolume={v => dispatch(setRowVolume(row.id, v))}
        pan={row.pan}
        onPan={p => dispatch(setRowPan(row.id, p))}
        removeTitle='Remove the row, and its notes from every pattern'
        onRemove={() => dispatch(removeRow(row.id))}
      />
      <div className={`seq-steps relative ${row.muted ? 'is-muted' : ''}`} data-keys={keys}>
        {litSteps
          ? beats()
          : <MiniNotes name={name} notes={notes} length={length} beatsPerBar={beatsPerBar} muted={row.muted} />}
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
