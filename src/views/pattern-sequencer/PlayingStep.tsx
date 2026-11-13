import React from 'react'
import { PatternRun } from '../../state/slices/sequencerSlice'
import { stepAt } from '../../types/sequencer'
import { GRID_LEFT, stepX } from './stepGrid'

interface Props {
  run: PatternRun | null
  // the pattern shown, and its length in steps
  patternId: number
  length: number
  beatsPerBar: number
}

// the step playing, while the pattern shown plays on its own: one column
// over every row's keys, moved to each new step by its transform, a frame
// at a time. nothing renders as it moves: the rows never hear of it
const PlayingStep = ({ run, patternId, length, beatsPerBar }: Props) => {
  const ref = React.useRef<HTMLDivElement>(null)
  const playing = !!run && run.patternId === patternId && length > 0

  React.useEffect(() => {
    const el = ref.current
    if(!el || !run || !playing) return undefined
    let frame = 0
    let shown: number | null = null
    const follow = () => {
      const step = stepAt(Date.now() - run.startTimeMs, { length }, run.beatIntervalMs)
      if(step !== null && step !== shown) {
        shown = step
        el.style.transform = `translateX(${stepX(step, beatsPerBar)}px)`
      }
      frame = requestAnimationFrame(follow)
    }
    follow()
    return () => cancelAnimationFrame(frame)
  }, [run, playing, length, beatsPerBar])

  return <div ref={ref} className='seq-now' style={{ left: GRID_LEFT }} hidden={!playing} aria-hidden='true' />
}

export default React.memo(PlayingStep)
