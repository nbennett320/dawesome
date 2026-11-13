import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern,
  loadSequencer,
  moveRow,
  selectPatternRun,
  selectRows,
  selectSelectedPattern,
  selectUsedOnly,
  setPatternStep,
  showNotes,
} from '../../state/slices/sequencerSlice'
import {
  Pattern, Row, greyedSteps, rowName, rowNotes, stepsPerBar,
} from '../../types/sequencer'
import { AnalogButton } from '../../components/fourtrack'
import { useTransport } from '../transport/useTransport'
import PatternRow from './PatternRow'
import PatternStrip from './PatternStrip'
import PlayingStep from './PlayingStep'
import { Paint } from './StepKeys'
import { GRID_LEFT } from './stepGrid'
import { RowDrop, itemName, useRowDrops } from './useRowDrops'
import { indexAmongAll, useRowDrag } from './useRowDrag'
import { useStopWhenUnfocused } from './useStopWhenUnfocused'
import { useVisibleBars } from './useVisibleBars'
import './steps.scss'

// the scroller's padding: the rows start this far into what it scrolls
const SCROLLER_PAD = 8

// painting steps ends wherever the mouse is let go. kept here, not in the
// rows, so a stroke carries on into bars that come into view as it goes.
// the tab renders when a stroke ends (see useShownRows)
const usePaint = () => {
  const paint = React.useRef<Paint | null>(null)
  const [, ended] = React.useReducer((strokes: number) => strokes + 1, 0)
  React.useEffect(() => {
    const end = () => {
      if(!paint.current) return
      paint.current = null
      ended()
    }
    window.addEventListener('mouseup', end)
    window.addEventListener('blur', end)
    return () => {
      window.removeEventListener('mouseup', end)
      window.removeEventListener('blur', end)
    }
  }, [])
  return paint
}

// the rows shown: every row, or with "Used only" those with notes in the
// pattern. while a stroke paints steps (`paint`), they stay as they were
// when it started, and are worked out again when it ends: a row whose last
// note it clears stays under the mouse, rather than going and sliding the
// rows below up under it, to have their steps cleared too
const useShownRows = (pattern: Pattern | null, paint: React.RefObject<Paint | null>): Row[] => {
  const rows = useAppSelector(selectRows)
  const usedOnly = useAppSelector(selectUsedOnly)
  const used = React.useMemo(
    () => (usedOnly && pattern ? rows.filter(row => pattern.notes[row.id] !== undefined) : rows),
    [rows, usedOnly, pattern],
  )
  const shown = React.useRef(used)
  if(!paint.current) shown.current = used
  return shown.current
}

// what the box under the rows says
const hintFor = (drop: RowDrop | null, rows: Row[]): string => {
  if(drop) {
    const replaced = rows.find(row => row.id === drop.rowId)
    const what = drop.item.kind === 'sample' ? 'sample' : 'instrument'
    return replaced
      ? `Let go to replace ${rowName(replaced)}’s ${what} with ${itemName(drop.item)}`
      : `Let go to add ${itemName(drop.item)}`
  }
  return rows.length
    ? 'Drop more samples or instruments here to add rows'
    : 'Drag samples or instruments here from the sidebar. Each one becomes a row'
}

// the pattern sequencer: the patterns along the top, then a row for each
// instrument, sample or synth, in one list (dragged into any order by their
// grips), with its 16th note steps in the selected pattern, or a picture of
// its notes when they aren't plain steps. only the bars in view are made,
// and the step playing is one column moved over them, so a long pattern
// costs what's on screen. the top bar's Play loops the selected pattern
// while this tab is focused
const PatternSequencer = () => {
  const dispatch = useAppDispatch()
  const pattern = useAppSelector(selectSelectedPattern)
  const run = useAppSelector(selectPatternRun)
  const allRows = useAppSelector(selectRows)
  const paint = usePaint()
  const rows = useShownRows(pattern, paint)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4
  const bar = stepsPerBar(beatsPerBar)
  const tabRef = React.useRef<HTMLDivElement>(null)
  const scrollerRef = React.useRef<HTMLDivElement>(null)

  // the engine owns the sequencer: it's caught up with when the tab opens.
  // a pattern playing on its own stops when focus leaves this tab and the
  // Piano Roll
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])
  useStopWhenUnfocused()

  const length = pattern?.length ?? 0
  const shownSteps = length + greyedSteps(length, bar)
  const view = useVisibleBars(scrollerRef, SCROLLER_PAD + GRID_LEFT, beatsPerBar, Math.ceil(shownSteps / bar))
  const drop = useRowDrops(tabRef)

  const patternId = pattern?.id
  const onStep = React.useCallback((rowId: number, step: number, on: boolean) => {
    if(patternId !== undefined) dispatch(setPatternStep(patternId, rowId, step, on, bar))
  }, [dispatch, patternId, bar])
  const onOpenNotes = React.useCallback((rowId: number) => {
    if(patternId !== undefined) dispatch(showNotes(patternId, rowId))
  }, [dispatch, patternId])

  // rows dragged by their grips to move them: among all the rows, when only
  // some are shown
  const rowIds = React.useMemo(() => rows.map(r => r.id), [rows])
  const allIds = React.useMemo(() => allRows.map(r => r.id), [allRows])
  const onMove = React.useCallback((rowId: number, index: number) => {
    dispatch(moveRow(rowId, indexAmongAll(allIds, rowIds, rowId, index))).catch(() => {})
  }, [dispatch, allIds, rowIds])
  const { draggingId, line, onGripDown } = useRowDrag(tabRef, rowIds, onMove)
  // how the keys look (a preference, see steps.scss), for every row at once
  const keys = useAppSelector(state => state.appearance.preferences.stepKeys)

  return (
    <div ref={tabRef} className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <PatternStrip beatsPerBar={beatsPerBar} playingId={run?.patternId ?? null} />

      <div
        ref={scrollerRef}
        data-testid='pattern-sequencer-rows'
        className={`flex-1 min-h-0 overflow-auto m-2 rounded-lg p-2 ft-tray ${drop ? 'ring-2 ring-accent-focus' : ''}`}
      >
        <div className='flex flex-col gap-1.5 w-max min-w-full'>
          {pattern ? (
            <div className='relative flex flex-col gap-1.5' data-keys={keys}>
              {rows.map(row => (
                <PatternRow
                  key={row.id}
                  row={row}
                  notes={rowNotes(pattern, row.id)}
                  length={pattern.length}
                  beatsPerBar={beatsPerBar}
                  fromBar={view.from}
                  toBar={view.to}
                  paint={paint}
                  onStep={onStep}
                  onOpenNotes={onOpenNotes}
                  dragging={row.id === draggingId}
                  replacingWith={drop?.rowId === row.id ? itemName(drop.item) : null}
                  dropLine={line?.rowId === row.id ? line.edge : null}
                  onGripDown={onGripDown}
                />
              ))}
              <PlayingStep run={run} patternId={pattern.id} length={pattern.length} beatsPerBar={beatsPerBar} />
            </div>
          ) : (
            <div className='flex flex-row items-center gap-3 px-1 py-2'>
              <p className='text-xs text-ink-soft'>No pattern yet</p>
              <AnalogButton
                variant='text'
                size='sm'
                title='A new, empty pattern'
                onPress={() => dispatch(addPattern(1))}
              >
                Make one
              </AnalogButton>
            </div>
          )}
          {/* (kept in view as the rows scroll sideways) */}
          <div
            className={`sticky left-0 rounded-md border-2 border-dashed px-3 py-4 text-xs text-center ${
              drop ? 'border-accent-focus text-accent-ink bg-accent-tint' : 'border-line-strong text-ink-soft'
            }`}
            style={view.width ? { maxWidth: view.width - 2 * SCROLLER_PAD } : undefined}
          >
            {hintFor(drop, rows)}
          </div>
        </div>
      </div>
    </div>
  )
}

export default PatternSequencer
