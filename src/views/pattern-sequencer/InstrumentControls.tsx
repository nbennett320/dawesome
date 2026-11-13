import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import { openPlugin } from '../../state/slices/effectsSlice'
import {
  previewRow,
  removeRow,
  renameRow,
  setRowGated,
  setRowMuted,
  setRowPan,
  setRowSamplePitch,
  setRowStepPitch,
  setRowVolume,
} from '../../state/slices/sequencerSlice'
import { hasTab } from '../../types/effects'
import { Row, SynthInstrument, rowName } from '../../types/sequencer'
import { parsePitch, pitchName } from '../piano-roll/notes'
import RowControls, { RowMenuItem } from './RowControls'

// the longest a row's name can be (as the engine keeps it), and a pitch's
const MAX_NAME_CHARS = 40
const MAX_PITCH_CHARS = 4

// a menu item opening a field for a pitch, typed as a name ("C4") or a
// number. a pitch it can't read, or the one it is, changes nothing
const pitchItem = (label: string, title: string, pitch: number, onPitch: (pitch: number) => void): RowMenuItem => ({
  label: `${label}…`,
  title: `${title}: ${pitchName(pitch)}`,
  field: {
    label,
    initial: pitchName(pitch),
    placeholder: pitchName(pitch),
    maxLength: MAX_PITCH_CHARS,
    onDone: text => {
      const typed = parsePitch(text)
      if(typed !== null && typed !== pitch) onPitch(typed)
    },
  },
})

// what clicking a synth's name says it does
const synthTitle = (synth: SynthInstrument): string => {
  if(synth.missing) return `${synth.name} isn’t installed. Its notes and settings are kept, and it’s silent`
  const by = synth.vendor ? ` by ${synth.vendor}` : ''
  return hasTab(synth)
    ? `${synth.name}${by}. Click to open its interface, right-click for more`
    : `${synth.name}${by}. It has no interface to show`
}

// a row's controls (see RowControls), worked by its instrument. both kinds
// open their notes in the Piano Roll, rename and have a step pitch. a
// sample's name plays it, and its menu sets its sample pitch and gate; a
// synth's name opens its plugin's interface. memoised: it renders again only
// when the row changes
const InstrumentControls = ({ row, onGripDown, onOpenNotes }: {
  row: Row
  onGripDown: (rowId: number, ev: React.MouseEvent) => void
  // opens the Piano Roll on the row's notes in the pattern shown
  onOpenNotes: (rowId: number) => void
}) => {
  const dispatch = useAppDispatch()
  const { instrument } = row
  // what it's called without its label
  const own = rowName({ ...row, label: null })

  // (a row showing steps has no picture of its notes to click, so this is
  // its way into the Piano Roll)
  const editNotes: RowMenuItem = {
    label: 'Edit in Piano Roll',
    title: 'Open the Piano Roll on this row’s notes in the pattern',
    onSelect: () => onOpenNotes(row.id),
  }
  const rename: RowMenuItem = {
    label: 'Rename…',
    field: {
      label: 'Row name',
      initial: row.label ?? '',
      placeholder: own,
      maxLength: MAX_NAME_CHARS,
      onDone: text => dispatch(renameRow(row.id, text)),
    },
  }
  const stepPitch = pitchItem('Step pitch', 'What a clicked step plays', row.stepPitch, pitch => {
    dispatch(setRowStepPitch(row.id, pitch))
  })
  const shared = {
    name: rowName(row),
    onGripDown: (ev: React.MouseEvent) => onGripDown(row.id, ev),
    muted: row.muted,
    onMute: (muted: boolean) => { dispatch(setRowMuted(row.id, muted)) },
    volume: row.volume,
    onVolume: (volume: number) => { dispatch(setRowVolume(row.id, volume)) },
    pan: row.pan,
    onPan: (pan: number) => { dispatch(setRowPan(row.id, pan)) },
    removeTitle: 'Remove the row, and its notes from every pattern',
    onRemove: () => { dispatch(removeRow(row.id)) },
  }

  if(instrument.kind === 'sample') {
    const sample = instrument
    return (
      <RowControls
        {...shared}
        note={row.label ? own : undefined}
        nameTitle={`${sample.path}. Click to hear it, right-click for more`}
        onName={() => { dispatch(previewRow(row.id)) }}
        menu={[
          editNotes,
          rename,
          stepPitch,
          pitchItem('Sample pitch', 'The note that plays the sample as it is', sample.samplePitch, pitch => {
            dispatch(setRowSamplePitch(row.id, pitch))
          }),
          {
            label: sample.gated ? '✓ Gated' : 'Gated',
            title: sample.gated
              ? 'Notes stop the sample at their ends. Choose to play it out (one-shot)'
              : 'Notes play the whole sample (one-shot). Choose to stop it at their ends',
            onSelect: () => { dispatch(setRowGated(row.id, !sample.gated)) },
          },
        ]}
      />
    )
  }

  const synth = instrument
  return (
    <RowControls
      {...shared}
      note={synth.missing ? 'Not installed' : undefined}
      disabled={synth.missing}
      nameTitle={synthTitle(synth)}
      onName={() => { if(hasTab(synth)) dispatch(openPlugin(synth.slot)).catch(() => {}) }}
      menu={[editNotes, rename, stepPitch]}
    />
  )
}

export default React.memo(InstrumentControls)
