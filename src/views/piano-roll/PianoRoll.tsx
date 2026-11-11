import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern,
  loadSequencer,
  selectInstrumentTracks,
  selectPattern,
  selectPatternRun,
  selectPatterns,
  selectPlaysOn,
  selectSelectedPattern,
  setPlaysOn,
  togglePattern,
} from '../../state/slices/sequencerSlice'
import { fetchEffects } from '../../state/slices/effectsSlice'
import { trackLabel } from '../../types/mixer'
import { Pattern, patternHint, patternLabel, stepsPerBar } from '../../types/sequencer'
import { AnalogButton, Led, PlayIcon, StopIcon } from '../../components/fourtrack'
import { useTransport } from '../transport/useTransport'
import { useStopWhenUnfocused } from '../sequencer/useStopWhenUnfocused'
import NoteGrid from './NoteGrid'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// a native drop-down with its label above it, like the strip's other parts
const Picker = ({ label, title, value, onChange, disabled = false, children }: {
  label: string
  title: string
  value: string
  onChange: (value: string) => void
  disabled?: boolean
  children: React.ReactNode
}) => (
  <div className='flex flex-col gap-1'>
    <span className='ft-label'>{label}</span>
    <select
      className='ft-readout text-xs py-1 text-left w-36 min-w-0 font-sans'
      aria-label={label}
      title={title}
      value={value}
      disabled={disabled}
      onChange={e => onChange(e.currentTarget.value)}
    >
      {children}
    </select>
  </div>
)

// the strip along the top: which pattern, which track's instrument plays
// it, play and stop, and how long it is
const Toolbar = ({ pattern, beatsPerBar }: { pattern: Pattern, beatsPerBar: number }) => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatterns)
  const tracks = useAppSelector(selectInstrumentTracks)
  const playsOn = useAppSelector(selectPlaysOn)
  const names = useAppSelector(state => state.mixer.names)
  const run = useAppSelector(selectPatternRun)
  const playing = run?.patternId === pattern.id

  return (
    <div className='ft-tray rounded-lg m-2 mb-0 p-2 flex flex-row flex-wrap items-end gap-x-4 gap-y-2'>
      <Picker
        label='Pattern'
        title='The pattern whose notes are shown (the Sequencer shows it too)'
        value={String(pattern.id)}
        onChange={value => dispatch(selectPattern(Number(value)))}
      >
        {patterns.map(p => <option key={p.id} value={p.id}>{patternLabel(p)}</option>)}
      </Picker>

      <Picker
        label='Plays on'
        title='The track whose instrument plays the notes, here and when the pattern plays on its own'
        value={playsOn === null ? '' : String(playsOn)}
        disabled={!tracks.length}
        onChange={value => dispatch(setPlaysOn(Number(value)))}
      >
        {tracks.length
          ? tracks.map(track => <option key={track} value={track}>{names[track] ?? trackLabel(track)}</option>)
          : <option value=''>No instrument</option>}
      </Picker>

      <div className='flex flex-col gap-1'>
        <span className='ft-label'>Play</span>
        <div className='flex flex-row items-center gap-1.5'>
          <Led color='lime' state={playing ? 'on' : 'off'} />
          <AnalogButton
            size='sm'
            latched={playing}
            aria-label={playing ? 'Stop the pattern' : 'Play the pattern'}
            title={playing ? 'Stop' : 'Loop this pattern on its own'}
            onPress={() => { dispatch(togglePattern(pattern.id)).catch(logError('play the pattern')) }}
          >
            {playing ? <StopIcon /> : <PlayIcon />}
          </AnalogButton>
        </div>
      </div>

      <div className='flex flex-col gap-1'>
        <span className='ft-label'>Length</span>
        <span className='text-xs text-ink-mid h-7 leading-7 tabular-nums'>{patternHint(pattern, beatsPerBar, 0)}</span>
      </div>

      {!tracks.length && (
        <p className='ml-auto self-center text-xs text-ink-faint'>
          Drag an instrument onto a track in the Mixer to hear notes
        </p>
      )}
    </div>
  )
}

// the selected pattern's notes on a grid of 16th note steps, against a
// piano keyboard. the pattern is the Sequencer's selected one, so the two
// tabs agree, and Play loops it while either is focused
const PianoRoll = () => {
  const dispatch = useAppDispatch()
  const pattern = useAppSelector(selectSelectedPattern)
  const run = useAppSelector(selectPatternRun)
  const track = useAppSelector(selectPlaysOn)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4

  useStopWhenUnfocused()

  // the engine owns the patterns and the instruments: catch up with it when
  // the tab opens
  React.useEffect(() => {
    dispatch(loadSequencer())
    dispatch(fetchEffects()).catch(logError('read the effects'))
  }, [dispatch])

  if(!pattern) {
    return (
      <div className='ft-panel h-full w-full flex flex-col items-center justify-center gap-3 text-sm select-none'>
        <p className='text-xs text-ink-soft'>No pattern yet. Notes are drawn into a pattern</p>
        <AnalogButton variant='text' size='sm' title='A new, empty pattern' onPress={() => dispatch(addPattern(1))}>
          New pattern
        </AnalogButton>
      </div>
    )
  }

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <Toolbar pattern={pattern} beatsPerBar={beatsPerBar} />
      <div className='flex-1 min-h-0 flex flex-col m-2 rounded-lg p-0.5 ft-tray'>
        <NoteGrid pattern={pattern} stepsPerBar={stepsPerBar(beatsPerBar)} track={track} run={run} />
      </div>
    </div>
  )
}

export default PianoRoll
