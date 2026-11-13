import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  PatternRun,
  addSequencerRow,
  moveSequencerRow,
  selectSequencerRows,
  setPatternStep,
} from '../../state/slices/sequencerSlice'
import { DragItem } from '../../state/sampleDrag'
import { Pattern, stepAt } from '../../types/sequencer'
import SequencerFrame, { useDropOn, useSequencerTab } from './SequencerFrame'
import SequencerRow, { Paint } from './SequencerRow'
import { useRowDrag } from './useRowDrag'

// the step of `pattern` playing now, or null when it isn't the pattern
// playing. followed every frame, but only a new step re-renders: rows
// re-render when the highlight moves, not in between
const usePlayingStep = (run: PatternRun | null, pattern: Pattern | null): number | null => {
  const [step, setStep] = React.useState<number | null>(null)

  React.useEffect(() => {
    if(!run || !pattern || run.patternId !== pattern.id) {
      setStep(null)
      return undefined
    }

    let frame = 0
    const follow = () => {
      // (the same step again doesn't re-render)
      setStep(stepAt(Date.now() - run.startTimeMs, pattern, run.beatIntervalMs))
      frame = requestAnimationFrame(follow)
    }
    follow()
    return () => cancelAnimationFrame(frame)
  }, [run, pattern])

  return step
}

const isSample = (item: DragItem): item is Extract<DragItem, { kind: 'sample' }> => item.kind === 'sample'

// the drum machine: drum patterns along the top, a row per sample, and a
// key for each 16th note step. Play loops the selected pattern while this
// tab is focused
const Sequencer = () => {
  const dispatch = useAppDispatch()
  const rows = useAppSelector(selectSequencerRows)
  const { pattern, run, beatsPerBar } = useSequencerTab('drum')
  const tabRef = React.useRef<HTMLDivElement>(null)
  // samples dragged from the sidebar become rows
  const dropping = useDropOn(tabRef, isSample, item => { dispatch(addSequencerRow(item.path)) })

  // painting steps ends wherever the mouse is let go
  const paint = React.useRef<Paint | null>(null)
  React.useEffect(() => {
    const end = () => { paint.current = null }
    window.addEventListener('mouseup', end)
    window.addEventListener('blur', end)
    return () => {
      window.removeEventListener('mouseup', end)
      window.removeEventListener('blur', end)
    }
  }, [])

  const onStep = React.useCallback((rowId: number, step: number, on: boolean) => {
    if(pattern) dispatch(setPatternStep(pattern.id, rowId, step, on))
  }, [dispatch, pattern?.id])

  // rows dragged by their grips to move them
  const rowIds = React.useMemo(() => rows.map(r => r.id), [rows])
  const onMove = React.useCallback((rowId: number, index: number) => {
    dispatch(moveSequencerRow(rowId, index)).catch(() => {})
  }, [dispatch])
  const { draggingId, line, onGripDown } = useRowDrag(tabRef, rowIds, onMove)

  // the step highlight follows the shown pattern while it plays
  const playingStep = usePlayingStep(run, pattern)

  let hint = 'Drop more samples here to add rows'
  if(!rows.length) hint = 'Drag samples here from the sidebar. Each one becomes a row of steps'
  if(dropping) hint = 'Let go to add the sample'

  return (
    <SequencerFrame
      kind='drum'
      pattern={pattern}
      beatsPerBar={beatsPerBar}
      playingId={run?.patternId ?? null}
      dropRef={tabRef}
      dropping={dropping}
      hint={hint}
    >
      {pattern && rows.map(row => (
        <SequencerRow
          key={row.id}
          row={row}
          pattern={pattern}
          beatsPerBar={beatsPerBar}
          playingStep={playingStep}
          paint={paint}
          onStep={onStep}
          dragging={row.id === draggingId}
          dropLine={line?.rowId === row.id ? line.edge : null}
          onGripDown={onGripDown}
        />
      ))}
    </SequencerFrame>
  )
}

export default Sequencer
