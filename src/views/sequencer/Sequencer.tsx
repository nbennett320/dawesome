import React from 'react'
import { useStore } from 'react-redux'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import {
  PatternRun,
  addSequencerRow,
  loadSequencer,
  selectPatternRun,
  selectSelectedPattern,
  selectSequencerRows,
  setPatternStep,
  stopPattern,
} from '../../state/slices/sequencerSlice'
import { selectFocusedView } from '../../state/slices/windowSlice'
import { onSampleDrag } from '../../state/sampleDrag'
import { Pattern, stepAt } from '../../types/sequencer'
import { View } from '../../types/ui'
import { useTransport } from '../transport/useTransport'
import PatternStrip from './PatternStrip'
import SequencerRow, { Paint } from './SequencerRow'

// a pattern playing on its own stops once the Sequencer isn't the focused
// pane's tab: another tab selected, another pane clicked, or the tab closed
const useStopWhenUnfocused = () => {
  const store = useStore<RootState>()
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    let stopping = false
    const stopIfPlaying = () => {
      if(stopping || !store.getState().sequencer.run) return
      stopping = true
      dispatch(stopPattern()).finally(() => { stopping = false })
    }

    const check = () => {
      if(selectFocusedView(store.getState()) !== View.Sequencer) stopIfPlaying()
    }

    const unsubscribe = store.subscribe(check)
    return () => {
      unsubscribe()
      stopIfPlaying()
    }
  }, [store, dispatch])
}

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

// samples dragged from the sidebar onto the sequencer become rows. returns
// whether one is being dragged over it now
const useSampleDrop = (ref: React.RefObject<HTMLElement>) => {
  const dispatch = useAppDispatch()
  const [over, setOver] = React.useState(false)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    const isOver = item.kind === 'sample'
      && !!ref.current?.contains(document.elementFromPoint(clientX, clientY))

    setOver(type === 'move' && isOver)
    if(type === 'drop' && isOver && item.kind === 'sample') dispatch(addSequencerRow(item.path))
  }), [dispatch, ref])

  return over
}

// the drum machine: patterns along the top, a row per sample, and a key for
// each 16th note step. Play (in the top bar) loops the selected pattern while
// this tab is focused
const Sequencer = () => {
  const dispatch = useAppDispatch()
  const rows = useAppSelector(selectSequencerRows)
  const pattern = useAppSelector(selectSelectedPattern)
  const run = useAppSelector(selectPatternRun)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4
  const bodyRef = React.useRef<HTMLDivElement>(null)
  const dropping = useSampleDrop(bodyRef)

  useStopWhenUnfocused()

  // the engine owns the sequencer: catch up with it when the tab opens
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])

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

  // the step highlight follows the shown pattern while it plays
  const playingStep = usePlayingStep(run, pattern)

  let hint = 'Drop more samples here to add rows'
  if(!rows.length) hint = 'Drag samples here from the sidebar. Each one becomes a row of steps'
  if(dropping) hint = 'Let go to add the sample'

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <PatternStrip beatsPerBar={beatsPerBar} playingId={run?.patternId ?? null} />

      <div
        ref={bodyRef}
        className={`flex-1 min-h-0 overflow-auto m-2 rounded-lg p-2 ft-tray ${dropping ? 'ring-2 ring-sky-400' : ''}`}
      >
        <div className='flex flex-col gap-1.5 w-max min-w-full'>
          {pattern && rows.map(row => (
            <SequencerRow
              key={row.id}
              row={row}
              pattern={pattern}
              beatsPerBar={beatsPerBar}
              playingStep={playingStep}
              paint={paint}
              onStep={onStep}
            />
          ))}
          <div
            className={`rounded-md border-2 border-dashed px-3 py-4 text-xs text-center ${
              dropping ? 'border-sky-400 text-sky-700 bg-sky-50' : 'border-slate-300 text-slate-500'
            }`}
          >
            {hint}
          </div>
        </div>
      </div>
    </div>
  )
}

export default Sequencer
