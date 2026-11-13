import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern, loadSequencer, selectPatternRun, selectSelectedPattern,
} from '../../state/slices/sequencerSlice'
import { DragItem, onSampleDrag } from '../../state/sampleDrag'
import { Pattern, PatternKind } from '../../types/sequencer'
import { AnalogButton } from '../../components/fourtrack'
import { useTransport } from '../transport/useTransport'
import PatternStrip from './PatternStrip'
import { useStopWhenUnfocused } from './useStopWhenUnfocused'

// what the Drum Sequencer and the Note Sequencer share: patterns of their
// kind along the top, a row for each sample or synth, and things dragged
// from the sidebar becoming rows

// things of one kind dragged from the sidebar onto `ref` are dropped there
// (`onDrop` is told where, in the window). returns whether one is being
// dragged over it now
export const useDropOn = <T extends DragItem>(
  ref: React.RefObject<HTMLElement>,
  accepts: (item: DragItem) => item is T,
  onDrop: (item: T, at: { clientX: number, clientY: number }) => void,
): boolean => {
  const [over, setOver] = React.useState(false)
  const drop = React.useRef(onDrop)
  drop.current = onDrop

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    const isOver = accepts(item) && !!ref.current?.contains(document.elementFromPoint(clientX, clientY))

    setOver(type === 'move' && isOver)
    if(type === 'drop' && isOver && accepts(item)) drop.current(item, { clientX, clientY })
  }), [ref, accepts])

  return over
}

// a sequencer tab's pattern, and the time signature's bar. the engine owns
// the sequencer: it's caught up with when the tab opens. a pattern playing
// on its own stops when focus leaves the sequencers and the Piano Roll
export const useSequencerTab = (kind: PatternKind) => {
  const dispatch = useAppDispatch()
  const pattern = useAppSelector(selectSelectedPattern(kind))
  const run = useAppSelector(selectPatternRun)
  const { transport } = useTransport()

  useStopWhenUnfocused()
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])

  return { pattern, run, beatsPerBar: transport?.beatsPerBar ?? 4 }
}

interface Props {
  kind: PatternKind
  pattern: Pattern | null
  beatsPerBar: number
  playingId: number | null
  // the whole tab, where things are dropped to become rows
  dropRef: React.RefObject<HTMLDivElement>
  // something that would become a row is being dragged over it
  dropping: boolean
  // what the box under the rows says
  hint: string
  // the rows, shown while there's a pattern
  children: React.ReactNode
}

// a sequencer's tab: its pattern strip, its rows, and a box under them to
// drop things in. with no pattern of its kind, it offers to make one
const SequencerFrame = ({ kind, pattern, beatsPerBar, playingId, dropRef, dropping, hint, children }: Props) => {
  const dispatch = useAppDispatch()

  return (
    <div ref={dropRef} className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <PatternStrip kind={kind} beatsPerBar={beatsPerBar} playingId={playingId} />

      <div
        className={`flex-1 min-h-0 overflow-auto m-2 rounded-lg p-2 ft-tray ${dropping ? 'ring-2 ring-accent-focus' : ''}`}
      >
        <div className='flex flex-col gap-1.5 w-max min-w-full'>
          {pattern ? children : (
            <div className='flex flex-row items-center gap-3 px-1 py-2'>
              <p className='text-xs text-ink-soft'>No pattern yet</p>
              <AnalogButton
                variant='text'
                size='sm'
                title='A new, empty pattern'
                onPress={() => dispatch(addPattern(1, kind))}
              >
                Make one
              </AnalogButton>
            </div>
          )}
          <div
            className={`rounded-md border-2 border-dashed px-3 py-4 text-xs text-center ${
              dropping ? 'border-accent-focus text-accent-ink bg-accent-tint' : 'border-line-strong text-ink-soft'
            }`}
          >
            {hint}
          </div>
        </div>
      </div>
    </div>
  )
}

export default SequencerFrame
