import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import { openPlugin } from '../../state/slices/effectsSlice'
import {
  removeRow, setRowMuted, setRowVolume, showNotes,
} from '../../state/slices/sequencerSlice'
import { hasTab } from '../../types/effects'
import { Note, Row, SynthInstrument } from '../../types/sequencer'
import RowControls from '../sequencer/RowControls'
import MiniNotes from './MiniNotes'

interface Props {
  row: Row<SynthInstrument>
  patternId: number
  // its notes in the pattern
  notes: Note[]
  // the pattern's length, in steps
  length: number
  beatsPerBar: number
}

// what clicking a synth's name says it does
const nameTitle = (synth: SynthInstrument): string => {
  if(synth.missing) return `${synth.name} isn’t installed. Its notes and settings are kept, and it’s silent`
  const by = synth.vendor ? ` by ${synth.vendor}` : ''
  return hasTab(synth) ? `${synth.name}${by}. Click to open its interface` : `${synth.name}${by}. It has no interface to show`
}

// one synth row: its synth's name (click for its own interface, which goes
// by the plugin's slot), mute, volume and remove, then a picture of its
// notes in the pattern (click for the Piano Roll on them)
const SynthRow = ({ row, patternId, notes, length, beatsPerBar }: Props) => {
  const dispatch = useAppDispatch()
  const synth = row.instrument

  return (
    <div className='flex flex-row items-center gap-2'>
      <RowControls
        name={synth.name}
        nameTitle={nameTitle(synth)}
        onName={() => { if(hasTab(synth)) dispatch(openPlugin(synth.slot)).catch(() => {}) }}
        disabled={synth.missing}
        note={synth.missing ? 'Not installed' : undefined}
        muted={row.muted}
        onMute={muted => { dispatch(setRowMuted(row.id, muted)).catch(() => {}) }}
        volume={row.volume}
        onVolume={v => dispatch(setRowVolume(row.id, v))}
        removeTitle='Remove the synth, and its notes from every pattern'
        onRemove={() => { dispatch(removeRow(row.id)).catch(() => {}) }}
      />
      <MiniNotes
        name={synth.name}
        notes={notes}
        length={length}
        beatsPerBar={beatsPerBar}
        muted={row.muted || synth.missing}
        onOpen={() => dispatch(showNotes(patternId, row.id))}
      />
    </div>
  )
}

export default React.memo(SynthRow)
