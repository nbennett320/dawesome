import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern,
  loadSequencer,
  selectPattern,
  selectPatternRun,
  selectPatterns,
  selectPianoRollRow,
  selectSelectedPattern,
  selectSynthRows,
  setPianoRollRow,
  togglePattern,
} from '../../state/slices/sequencerSlice'
import {
  selectPianoRollSnapEnabled, selectPianoRollSnapNote, selectPianoRollStart, selectPianoRollTool,
  setPianoRollSnapNote, setPianoRollTool, togglePianoRollSnap,
} from '../../state/slices/pianoRollSlice'
import { openTab } from '../../state/slices/windowSlice'
import { TOOLS } from '../../editing/tools'
import ToolTray from '../../components/editing/ToolTray'
import {
  Note, Pattern, Row, SynthInstrument, patternHint, patternLabel, rowName, stepsPerBar,
} from '../../types/sequencer'
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

// the strip along the top: which pattern and which synth row's notes, play
// and stop, how long it is, and the mouse tools
const Toolbar = ({ pattern, row, beatsPerBar }: {
  pattern: Pattern
  row: Row<SynthInstrument> | null
  beatsPerBar: number
}) => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatterns)
  const synthRows = useAppSelector(selectSynthRows)
  const run = useAppSelector(selectPatternRun)
  const playing = run?.patternId === pattern.id
  const startStep = useAppSelector(selectPianoRollStart(pattern.id))

  return (
    <div className='ft-tray rounded-lg m-2 mb-0 p-2 flex flex-row flex-wrap items-end gap-x-4 gap-y-2'>
      <Picker
        label='Pattern'
        title='The pattern whose notes are shown (the sequencers show it too)'
        value={String(pattern.id)}
        onChange={value => dispatch(selectPattern(Number(value)))}
      >
        {patterns.map(p => <option key={p.id} value={p.id}>{patternLabel(p)}</option>)}
      </Picker>

      {row && (
        <Picker
          label='Synth'
          title='The synth whose notes are edited, and which plays the keys'
          value={String(row.id)}
          onChange={value => dispatch(setPianoRollRow(Number(value)))}
        >
          {synthRows.map(r => <option key={r.id} value={r.id}>{rowName(r)}</option>)}
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
            title={playing ? 'Stop' : 'Loop this pattern on its own, every row’s notes, from the playhead'}
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

// one synth row's notes in a pattern, on a grid of 16th note steps against
// a piano keyboard. the pattern is the sequencers' selected one, so the tabs
// agree, and Play loops it while any of them is focused. the other rows'
// notes show faintly behind
const PianoRoll = () => {
  const dispatch = useAppDispatch()
  const pattern = useAppSelector(selectSelectedPattern)
  const row = useAppSelector(selectPianoRollRow)
  const run = useAppSelector(selectPatternRun)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4

  useStopWhenUnfocused()

  // the engine owns the patterns and the rows: catch up with it when the
  // tab opens
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])

  // every other row's notes, sample rows' too
  const byRow = pattern?.notes
  const rowId = row?.id
  const ghosts = React.useMemo(() => {
    if(!byRow || rowId === undefined) return NO_GHOSTS
    return Object.entries(byRow).filter(([id]) => Number(id) !== rowId).flatMap(([, notes]) => notes)
  }, [byRow, rowId])

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
      <Toolbar pattern={pattern} row={row} beatsPerBar={beatsPerBar} />
      {row ? (
        <div className='flex-1 min-h-0 flex flex-col m-2 rounded-lg p-0.5 ft-tray'>
          <NoteGrid
            pattern={pattern}
            rowId={row.id}
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
