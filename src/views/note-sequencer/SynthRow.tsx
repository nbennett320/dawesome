import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import { openPlugin } from '../../state/slices/effectsSlice'
import {
  removeSynth, setSynthMuted, setSynthVolume, showNotes,
} from '../../state/slices/sequencerSlice'
import { hasTab } from '../../types/effects'
import { Pattern, Synth, synthNotes } from '../../types/sequencer'
import RowControls from '../sequencer/RowControls'
import MiniNotes from './MiniNotes'

interface Props {
  synth: Synth
  pattern: Pattern
  beatsPerBar: number
}

// what clicking a synth's name says it does
const nameTitle = (synth: Synth): string => {
  if(synth.missing) return `${synth.name} isn’t installed. Its notes and settings are kept, and it’s silent`
  const by = synth.vendor ? ` by ${synth.vendor}` : ''
  return hasTab(synth) ? `${synth.name}${by}. Click to open its interface` : `${synth.name}${by}. It has no interface to show`
}

// one synth: its name (click for its own interface), mute, volume and
// remove, then a picture of its notes in the pattern (click for the Piano
// Roll on them)
const SynthRow = ({ synth, pattern, beatsPerBar }: Props) => {
  const dispatch = useAppDispatch()

  return (
    <div className='flex flex-row items-center gap-2'>
      <RowControls
        name={synth.name}
        nameTitle={nameTitle(synth)}
        onName={() => { if(hasTab(synth)) dispatch(openPlugin(synth.id)).catch(() => {}) }}
        disabled={synth.missing}
        note={synth.missing ? 'Not installed' : undefined}
        muted={synth.muted}
        onMute={muted => { dispatch(setSynthMuted(synth.id, muted)).catch(() => {}) }}
        volume={synth.volume}
        onVolume={v => dispatch(setSynthVolume(synth.id, v))}
        removeTitle='Remove the synth, and its notes from every pattern'
        onRemove={() => { dispatch(removeSynth(synth.id)).catch(() => {}) }}
      />
      <MiniNotes
        name={synth.name}
        notes={synthNotes(pattern, synth.id)}
        length={pattern.length}
        beatsPerBar={beatsPerBar}
        muted={synth.muted || synth.missing}
        onOpen={() => dispatch(showNotes(pattern.id, synth.id))}
      />
    </div>
  )
}

export default React.memo(SynthRow)
