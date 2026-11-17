import React from 'react'
import { usePlayhead } from '../transport/usePlayhead'
import { useElementSize } from '../transport/useElementSize'
import { formatTime } from '../transport/tape'
import { barTicks, loopBand, msToX, readout, timelineEndMs } from './strip'

// the Timeline tab: the whole song along one strip, from its start on the
// left to its end on the right, with the loop and the playhead on it
const Timeline = () => {
  const strip = React.useRef<HTMLDivElement>(null)
  const size = useElementSize(strip)
  const width = size?.width ?? 0
  const { transport, position, playing } = usePlayhead()

  const end = transport ? timelineEndMs(transport) : 0
  const x = (ms: number) => msToX(ms, end, width)
  const ticks = transport ? barTicks(transport, end, width) : []
  const loop = transport ? loopBand(transport) : null

  return (
    <div className='flex h-full w-full flex-col gap-1 bg-surface p-3'>
      <div
        ref={strip}
        className='relative min-h-0 flex-1 overflow-hidden rounded border border-line-strong bg-sunken'
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
            className={`absolute inset-y-0 w-0.5 -translate-x-1/2 ${playing ? 'bg-accent' : 'bg-ink-faint'}`}
            style={{ left: x(position) }}
          />
        )}
      </div>
      <div className='flex shrink-0 items-center justify-between text-[11px] tabular-nums text-ink-faint'>
        <span>{formatTime(0)}</span>
        <span className='text-ink-mid'>{transport ? readout(position, transport) : ''}</span>
        <span>{formatTime(end)}</span>
      </div>
    </div>
  )
}

export default Timeline
