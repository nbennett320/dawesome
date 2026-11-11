import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { addSynth, selectSynths } from '../../state/slices/sequencerSlice'
import { DragItem } from '../../state/sampleDrag'
import SequencerFrame, { useDropOn, useSequencerTab } from '../sequencer/SequencerFrame'
import SynthRow from './SynthRow'

const isInstrument = (item: DragItem): item is Extract<DragItem, { kind: 'instrument' }> =>
  item.kind === 'instrument'

// the Drum Sequencer's twin for notes: note patterns along the top, and a
// row per synth with a picture of its notes in the selected pattern, edited
// in the Piano Roll. Play loops the selected pattern while this tab is
// focused
const NoteSequencer = () => {
  const dispatch = useAppDispatch()
  const synths = useAppSelector(selectSynths)
  const { pattern, run, beatsPerBar } = useSequencerTab('note')
  const tabRef = React.useRef<HTMLDivElement>(null)
  // instruments dragged from the sidebar, anywhere on the tab, become synths
  const dropping = useDropOn(tabRef, isInstrument, item => { dispatch(addSynth(item.pluginId)).catch(() => {}) })

  let hint = 'Drop more instruments here to add synths'
  if(!synths.length) hint = 'Drag an instrument here'
  if(dropping) hint = 'Let go to add the instrument'

  return (
    <SequencerFrame
      kind='note'
      pattern={pattern}
      beatsPerBar={beatsPerBar}
      playingId={run?.patternId ?? null}
      dropRef={tabRef}
      dropping={dropping}
      hint={hint}
    >
      {pattern && synths.map(synth => (
        <SynthRow key={synth.id} synth={synth} pattern={pattern} beatsPerBar={beatsPerBar} />
      ))}
    </SequencerFrame>
  )
}

export default NoteSequencer
