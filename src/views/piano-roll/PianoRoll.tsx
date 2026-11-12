import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern,
  loadSequencer,
  selectPattern,
  selectPatternRun,
  selectPatternsOf,
  selectPianoRollSynth,
  selectSelectedPattern,
  selectSynths,
  setPianoRollSynth,
  togglePattern,
} from '../../state/slices/sequencerSlice'
import {
  selectPianoRollSnapEnabled, selectPianoRollSnapNote, selectPianoRollStart, selectPianoRollTool,
  setPianoRollSnapNote, setPianoRollTool, togglePianoRollSnap,
} from '../../state/slices/pianoRollSlice'
import { openTab } from '../../state/slices/windowSlice'
import { TOOLS } from '../../editing/tools'
import ToolTray from '../../components/editing/ToolTray'
import { Note, Pattern, Synth, patternHint, patternLabel, stepsPerBar } from '../../types/sequencer'
import { View } from '../../types/ui'
import { AnalogButton, Led } from '../../components/fourtrack'
import { useTransport } from '../transport/useTransport'
import { useStopWhenUnfocused } from '../sequencer/useStopWhenUnfocused'
import NoteGrid from './NoteGrid'
import { PlayIcon, StopIcon } from '../../components/icons'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// a native drop-down with its label above it, like the strip's other parts
const Picker = ({ label, title, value, onChange, children }: {
  label: string
  title: string
  value: string
  onChange: (value: string) => void
  children: React.ReactNode
}) => (
  <div className='flex flex-col gap-1'>
    <span className='ft-label'>{label}</span>
    <select
      className='ft-readout text-xs py-1 text-left w-36 min-w-0 font-sans'
      aria-label={label}
      title={title}
      value={value}
      onChange={e => onChange(e.currentTarget.value)}
    >
      {children}
    </select>
  </div>
)

// the mouse tools (Select, Draw, Cut) and snap, as the playlist has them
const PianoRollTray = () => {
  const dispatch = useAppDispatch()
  const tool = useAppSelector(selectPianoRollTool)
  const snapEnabled = useAppSelector(selectPianoRollSnapEnabled)
  const snapNote = useAppSelector(selectPianoRollSnapNote)

  return (
    <ToolTray
      view={View.PianoRoll}
      tools={TOOLS}
      tool={tool}
      onTool={next => dispatch(setPianoRollTool(next))}
      snap={{
        enabled: snapEnabled,
        note: snapNote,
        onToggle: () => { dispatch(togglePianoRollSnap()) },
        onNote: note => { dispatch(setPianoRollSnapNote(note)) },
        label: 'Piano Roll snap amount',
      }}
    />
  )
}

// the strip along the top: which note pattern and which synth's notes,
// play and stop, how long it is, and the mouse tools
const Toolbar = ({ pattern, synth, beatsPerBar }: { pattern: Pattern, synth: Synth | null, beatsPerBar: number }) => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatternsOf('note'))
  const synths = useAppSelector(selectSynths)
  const run = useAppSelector(selectPatternRun)
  const playing = run?.patternId === pattern.id
  const startStep = useAppSelector(selectPianoRollStart(pattern.id))

  return (
    <div className='ft-tray rounded-lg m-2 mb-0 p-2 flex flex-row flex-wrap items-end gap-x-4 gap-y-2'>
      <Picker
        label='Pattern'
        title='The note pattern whose notes are shown (the Note Sequencer shows it too)'
        value={String(pattern.id)}
        onChange={value => dispatch(selectPattern(Number(value)))}
      >
        {patterns.map(p => <option key={p.id} value={p.id}>{patternLabel(p)}</option>)}
      </Picker>

      {synth && (
        <Picker
          label='Synth'
          title='The synth whose notes are edited, and which plays the keys'
          value={String(synth.id)}
          onChange={value => dispatch(setPianoRollSynth(Number(value)))}
        >
          {synths.map(s => <option key={s.id} value={s.id}>{s.name}</option>)}
        </Picker>
      )}

      <div className='flex flex-col gap-1'>
        <span className='ft-label'>Play</span>
        <div className='flex flex-row items-center gap-1.5'>
          <Led color='lime' state={playing ? 'on' : 'off'} />
          <AnalogButton
            size='sm'
            latched={playing}
            aria-label={playing ? 'Stop the pattern' : 'Play the pattern'}
            title={playing ? 'Stop' : 'Loop this pattern on its own, every synth’s notes, from the playhead'}
            onPress={() => { dispatch(togglePattern(pattern.id, startStep)).catch(logError('play the pattern')) }}
          >
            {playing ? <StopIcon /> : <PlayIcon />}
          </AnalogButton>
        </div>
      </div>

      <div className='flex flex-col gap-1'>
        <span className='ft-label'>Length</span>
        <span className='text-xs text-ink-mid h-7 leading-7 tabular-nums'>{patternHint(pattern, beatsPerBar, 0)}</span>
      </div>

      <div className='ml-auto'>
        <PianoRollTray />
      </div>
    </div>
  )
}

// shown in place of the grid when there's nothing to edit yet
const Empty = ({ children }: { children: React.ReactNode }) => (
  <div className='flex-1 min-h-0 flex flex-col items-center justify-center gap-3 m-2 rounded-lg ft-tray'>
    {children}
  </div>
)

const NO_GHOSTS: Note[] = []

// one synth's notes in a note pattern, on a grid of 16th note steps against
// a piano keyboard. the pattern is the Note Sequencer's selected one, so the
// two tabs agree, and Play loops it while either is focused. the other
// synths' notes show faintly behind
const PianoRoll = () => {
  const dispatch = useAppDispatch()
  const pattern = useAppSelector(selectSelectedPattern('note'))
  const synth = useAppSelector(selectPianoRollSynth)
  const run = useAppSelector(selectPatternRun)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4

  useStopWhenUnfocused()

  // the engine owns the patterns and the synths: catch up with it when the
  // tab opens
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])

  const bySynth = pattern?.notes
  const synthId = synth?.id
  const ghosts = React.useMemo(() => {
    if(!bySynth || synthId === undefined) return NO_GHOSTS
    return Object.entries(bySynth).filter(([id]) => Number(id) !== synthId).flatMap(([, notes]) => notes)
  }, [bySynth, synthId])

  if(!pattern) {
    return (
      <div className='ft-panel h-full w-full flex flex-col items-center justify-center gap-3 text-sm select-none'>
        <p className='text-xs text-ink-soft'>No note pattern yet. Notes are drawn into a note pattern</p>
        <AnalogButton variant='text' size='sm' title='A new, empty note pattern' onPress={() => dispatch(addPattern(1, 'note'))}>
          New pattern
        </AnalogButton>
      </div>
    )
  }

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <Toolbar pattern={pattern} synth={synth} beatsPerBar={beatsPerBar} />
      {synth ? (
        <div className='flex-1 min-h-0 flex flex-col m-2 rounded-lg p-0.5 ft-tray'>
          <NoteGrid
            pattern={pattern}
            synthId={synth.id}
            ghosts={ghosts}
            stepsPerBar={stepsPerBar(beatsPerBar)}
            run={run}
          />
        </div>
      ) : (
        <Empty>
          <p className='text-xs text-ink-soft'>No synth yet. Drag an instrument into the Note Sequencer to draw its notes</p>
          <AnalogButton variant='text' size='sm' onPress={() => dispatch(openTab(View.NoteSequencer))}>
            Note Sequencer
          </AnalogButton>
        </Empty>
      )}
    </div>
  )
}

export default PianoRoll
