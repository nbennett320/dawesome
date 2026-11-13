import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  PatternRun,
  addSampleRow,
  moveRow,
  replaceRowSample,
  selectRows,
  selectSampleRows,
  setPatternStep,
} from '../../state/slices/sequencerSlice'
import { DragItem, onSampleDrag } from '../../state/sampleDrag'
import { Pattern, rowNotes, stepAt } from '../../types/sequencer'
import { fileName } from '../../util/helpers'
import SequencerFrame, { useDropOn, useSequencerTab } from './SequencerFrame'
import SequencerRow, { Paint } from './SequencerRow'
import { indexAmongAll, useRowDrag } from './useRowDrag'

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

// the row at a point in the window, inside `tab`, if there's one
const rowAt = (tab: HTMLElement | null, clientX: number, clientY: number): number | null => {
  const el = document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[data-row-id]')
  return el && tab?.contains(el) ? Number(el.dataset.rowId) : null
}

// a sample dragged over a row, to replace the row's sample with it
interface Replacing {
  rowId: number
  sample: string
}

// the row a sample from the sidebar is being dragged over now
const useReplacing = (tab: React.RefObject<HTMLElement>): Replacing | null => {
  const [replacing, setReplacing] = React.useState<Replacing | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    const rowId = type === 'move' && isSample(item) ? rowAt(tab.current, clientX, clientY) : null
    setReplacing(now => {
      if(rowId === null) return null
      if(now?.rowId === rowId) return now
      return { rowId, sample: fileName((item as Extract<DragItem, { kind: 'sample' }>).path) }
    })
  }), [tab])

  return replacing
}

// the drum machine: the patterns along the top, a row per sample row, and
// a key for each 16th note step. Play loops the selected pattern while this
// tab is focused
const Sequencer = () => {
  const dispatch = useAppDispatch()
  const allRows = useAppSelector(selectRows)
  const rows = useAppSelector(selectSampleRows)
  const { pattern, run, beatsPerBar } = useSequencerTab()
  const tabRef = React.useRef<HTMLDivElement>(null)
  // samples dragged from the sidebar become rows, or dropped on a row,
  // replace its sample (its steps and settings stay)
  const dropping = useDropOn(tabRef, isSample, (item, { clientX, clientY }) => {
    const rowId = rowAt(tabRef.current, clientX, clientY)
    if(rowId === null) dispatch(addSampleRow(item.path))
    else dispatch(replaceRowSample(rowId, item.path))
  })
  const replacing = useReplacing(tabRef)

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

  // rows dragged by their grips to move them. the synth rows between them
  // stay where they are
  const rowIds = React.useMemo(() => rows.map(r => r.id), [rows])
  const allIds = React.useMemo(() => allRows.map(r => r.id), [allRows])
  const onMove = React.useCallback((rowId: number, index: number) => {
    dispatch(moveRow(rowId, indexAmongAll(allIds, rowIds, rowId, index))).catch(() => {})
  }, [dispatch, allIds, rowIds])
  const { draggingId, line, onGripDown } = useRowDrag(tabRef, rowIds, onMove)

  // the step highlight follows the shown pattern while it plays
  const playingStep = usePlayingStep(run, pattern)

  let hint = 'Drop more samples here to add rows'
  if(!rows.length) hint = 'Drag samples here from the sidebar. Each one becomes a row of steps'
  if(dropping) hint = 'Let go to add the sample'
  const replaced = replacing && rows.find(r => r.id === replacing.rowId)
  if(dropping && replaced) {
    hint = `Let go to replace ${replaced.label ?? fileName(replaced.instrument.path)}’s sample with ${replacing.sample}`
  }

  return (
    <SequencerFrame
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
          notes={rowNotes(pattern, row.id)}
          length={pattern.length}
          beatsPerBar={beatsPerBar}
          playingStep={playingStep}
          paint={paint}
          onStep={onStep}
          dragging={row.id === draggingId}
          replacingWith={replacing?.rowId === row.id ? replacing.sample : null}
          dropLine={line?.rowId === row.id ? line.edge : null}
          onGripDown={onGripDown}
        />
      ))}
    </SequencerFrame>
  )
}

export default Sequencer
