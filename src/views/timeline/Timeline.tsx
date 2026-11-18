import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectSnapEnabled, selectSnapNote } from '../../state/slices/playlistSlice'
import { scrubBegin, scrubEnd, scrubTo } from '../../state/scrub'
import { usePlayhead } from '../transport/usePlayhead'
import { useElementSize } from '../transport/useElementSize'
import { formatTime } from '../transport/tape'
import { barTicks, loopBand, msToX, readout, seekBeat, timelineEndMs, xToMs } from './strip'

// the Timeline tab: the whole song along one strip, from its start on the
// left to its end on the right, with the loop and the playhead on it. a
// click or a drag moves the playhead: it follows the mouse on screen, the
// song's heard scrubbing along with it (playback pauses meanwhile), and it's
// left where it's let go, snapped (playing carries on from there)
const Timeline = () => {
  const dispatch = useAppDispatch()
  const strip = React.useRef<HTMLDivElement>(null)
  const size = useElementSize(strip)
  const width = size?.width ?? 0
  const {
    transport, position, playing, refresh, assume,
  } = usePlayhead()
  const snapEnabled = useAppSelector(selectSnapEnabled)
  const snapNote = useAppSelector(selectSnapNote)

  // where the mouse has the playhead while it's pressed, in ms, and where
  // it was before (where a drag taken away leaves it)
  const [dragMs, setDragMs] = React.useState<number | null>(null)
  const dragFrom = React.useRef(0)
  // where it was let go (snapped), kept on screen until the engine has it
  const [seekMs, setSeekMs] = React.useState<number | null>(null)
  // seeks not yet back from the engine: the last one to come back lets go
  const seeks = React.useRef(0)

  const end = transport ? timelineEndMs(transport) : 0
  const x = (ms: number) => msToX(ms, end, width)
  const ticks = transport ? barTicks(transport, end, width) : []
  const loop = transport ? loopBand(transport) : null
  const shown = dragMs ?? seekMs ?? position
  const moving = playing || dragMs !== null || seekMs !== null

  const mouseMs = (e: React.PointerEvent<HTMLDivElement>) =>
    xToMs(e.clientX - e.currentTarget.getBoundingClientRect().left - e.currentTarget.clientLeft, end, width)

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if(e.button !== 0 || !transport || width <= 0) return
    e.currentTarget.setPointerCapture(e.pointerId)
    const ms = mouseMs(e)
    dragFrom.current = position
    setDragMs(ms)
    scrubBegin(ms, 'timeline')
  }

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if(dragMs === null) return
    const ms = mouseMs(e)
    setDragMs(ms)
    scrubTo(ms)
  }

  // let go at `ms` (snapped to `beat`)
  const letGo = (ms: number, beat: number) => {
    setDragMs(null)
    setSeekMs(ms)
    if(!playing) assume(ms)

    seeks.current += 1
    Promise.resolve(dispatch(scrubEnd(beat)))
      .then(refresh)
      .finally(() => {
        seeks.current -= 1
        if(seeks.current === 0) setSeekMs(null)
      })
  }

  const onPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if(dragMs === null || !transport) return
    const beat = seekBeat(mouseMs(e), transport, { enabled: snapEnabled, note: snapNote })
    letGo(beat * transport.beatIntervalMs, beat)
  }

  // a drag the system took away goes nowhere: back where it was
  const onPointerCancel = () => {
    if(dragMs === null || !transport || transport.beatIntervalMs <= 0) return
    letGo(dragFrom.current, dragFrom.current / transport.beatIntervalMs)
  }

  return (
    <div className='flex h-full w-full flex-col gap-1 bg-surface p-3 select-none'>
      <div
        ref={strip}
        data-testid='timeline-strip'
        className='relative min-h-0 flex-1 cursor-pointer touch-none overflow-hidden rounded border border-line-strong bg-sunken'
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerCancel}
      >
        {loop && (
          <div
            className={loop.bright
              ? 'absolute inset-y-0 border-x border-accent bg-accent/20'
              : 'absolute inset-y-0 border border-dashed border-accent/40'}
            style={{ left: x(loop.start), width: Math.max(1, x(loop.end) - x(loop.start)) }}
          />
        )}
        {ticks.map(ms => (
          <div key={ms} className='absolute inset-y-0 w-px bg-line' style={{ left: x(ms) }} />
        ))}
        {transport && width > 0 && (
          <div
            className={`pointer-events-none absolute inset-y-0 w-0.5 -translate-x-1/2 ${moving ? 'bg-accent' : 'bg-ink-faint'}`}
            style={{ left: x(shown) }}
          />
        )}
      </div>
      <div className='flex shrink-0 items-center justify-between text-[11px] tabular-nums text-ink-faint'>
        <span>{formatTime(0)}</span>
        <span className='text-ink-mid'>{transport ? readout(shown, transport) : ''}</span>
        <span>{formatTime(end)}</span>
      </div>
    </div>
  )
}

export default Timeline
