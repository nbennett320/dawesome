import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { addSynthRow, selectSynthRows } from '../../state/slices/sequencerSlice'
import { DragItem } from '../../state/sampleDrag'
import { rowNotes } from '../../types/sequencer'
import SequencerFrame, { useDropOn, useSequencerTab } from '../sequencer/SequencerFrame'
import SynthRow from './SynthRow'

const isInstrument = (item: DragItem): item is Extract<DragItem, { kind: 'instrument' }> =>
  item.kind === 'instrument'

// the Drum Sequencer's twin for synths: the patterns along the top, and a
// row per synth row with a picture of its notes in the selected pattern,
// edited in the Piano Roll. Play loops the selected pattern while this tab
// is focused
const NoteSequencer = () => {
  const dispatch = useAppDispatch()
  const synthRows = useAppSelector(selectSynthRows)
  const { pattern, run, beatsPerBar } = useSequencerTab()
  const tabRef = React.useRef<HTMLDivElement>(null)
  // instruments dragged from the sidebar, anywhere on the tab, become synth
  // rows
  const dropping = useDropOn(tabRef, isInstrument, item => { dispatch(addSynthRow(item.pluginId)).catch(() => {}) })

  let hint = 'Drop more instruments here to add synths'
  if(!synthRows.length) hint = 'Drag an instrument here'
  if(dropping) hint = 'Let go to add the instrument'

  return (
    <SequencerFrame
      pattern={pattern}
      beatsPerBar={beatsPerBar}
      playingId={run?.patternId ?? null}
      dropRef={tabRef}
      dropping={dropping}
      hint={hint}
    >
      {pattern && synthRows.map(row => (
        <SynthRow
          key={row.id}
          row={row}
          patternId={pattern.id}
          notes={rowNotes(pattern, row.id)}
          length={pattern.length}
          beatsPerBar={beatsPerBar}
        />
      ))}
    </SequencerFrame>
  )
}

export default NoteSequencer
