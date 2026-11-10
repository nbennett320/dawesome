import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import {
  previewSequencerRow,
  removeSequencerRow,
  setSequencerRowMuted,
  setSequencerRowVolume,
} from '../../state/slices/sequencerSlice'
import {
  Pattern,
  ROW_VOLUME_MAX,
  STEPS_PER_BEAT,
  SequencerRow as Row,
  fileName,
  isLit,
} from '../../types/sequencer'
import { AnalogButton, Knob, Led } from '../../components/fourtrack'

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
}

// a volume as a percentage of the sample's own level
const percent = (volume: number) => `${Math.round(volume * 100)}%`

// one sample: its name (click to hear it), mute, volume and remove, then a
// key for each of the pattern's steps
const SequencerRow = ({ row, pattern, beatsPerBar, playingStep, paint, onStep }: Props) => {
  const dispatch = useAppDispatch()
  const name = fileName(row.path)
  const stepsPerBar = Math.max(1, beatsPerBar) * STEPS_PER_BEAT

  const steps = Array.from({ length: pattern.length }, (_, step) => {
    const lit = isLit(pattern, row.id, step)
    const beat = Math.floor(step / STEPS_PER_BEAT)
    const firstOfBeat = step % STEPS_PER_BEAT === 0
    const firstOfBar = step % stepsPerBar === 0

    let tone = 'bg-white'
    if(lit) tone = row.muted ? 'bg-sky-300' : 'bg-sky-500'
    // every other beat a little darker, so the beats read at a glance
    else if(beat % 2 === 1) tone = 'bg-slate-100'

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
          lit ? 'border-sky-600 shadow-inner' : 'border-slate-300 hover:border-slate-400',
          step === playingStep ? 'ring-2 ring-amber-400' : '',
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
    <div className='flex flex-row items-center gap-2'>
      <div className='ft-strip shrink-0 w-64 h-11 flex flex-row items-center gap-1.5 px-1.5'>
        <button
          type='button'
          className={`ft-scribble flex-1 min-w-0 truncate text-left text-xs font-medium px-1 py-0.5 hover:bg-slate-50 ${row.muted ? 'text-slate-400 italic' : ''}`}
          title={`${row.path}. Click to hear it`}
          onClick={() => dispatch(previewSequencerRow(row.id))}
        >
          {name}
        </button>
        <div className='flex flex-col items-center gap-0.5'>
          <Led color='amber' state={row.muted ? 'on' : 'off'} />
          <AnalogButton
            variant='text'
            size='xs'
            latched={row.muted}
            aria-label={`Mute ${name}`}
            title='Mute'
            onPress={() => dispatch(setSequencerRowMuted(row.id, !row.muted))}
          >
            M
          </AnalogButton>
        </div>
        <Knob
          size={26}
          min={0}
          max={ROW_VOLUME_MAX}
          center={1}
          step={0.01}
          value={row.volume}
          onChange={v => dispatch(setSequencerRowVolume(row.id, v))}
          label={`${name} volume`}
          formatValue={percent}
          title={`Volume ${percent(row.volume)}. Double-click for 100%`}
        />
        <AnalogButton
          variant='text'
          size='xs'
          aria-label={`Remove ${name}`}
          title='Remove the row, and its steps from every pattern'
          onPress={() => dispatch(removeSequencerRow(row.id))}
        >
          ×
        </AnalogButton>
      </div>
      <div className='flex flex-row items-center'>{steps}</div>
    </div>
  )
}

export default React.memo(SequencerRow)
