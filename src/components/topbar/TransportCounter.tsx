import React from 'react'
import { RollingCounter, TapeCounter } from '../fourtrack'
import { barAndBeat, formatTime } from '../../views/transport/tape'
import { usePlayhead } from '../../views/transport/usePlayhead'

// where the playhead is: the bar and time on rolling counter wheels, and the
// beat on a still one (rolling it every beat would distract)
const TransportCounter = () => {
  const { transport, position } = usePlayhead()
  const { bar, beat } = transport ? barAndBeat(position, transport) : { bar: 1, beat: 1 }

  return (
    <div className='ft-lcd flex flex-row items-center gap-2 py-1'>
      <span className='ft-label text-[9px]'>Bar</span>
      <RollingCounter value={String(bar % 1000).padStart(3, '0')} label='Bar' />
      <span className='ft-label text-[9px] ml-1'>Beat</span>
      <TapeCounter value={beat} digits={1} label='Beat' />
      <span className='ft-label text-[9px] ml-1'>Time</span>
      <RollingCounter value={formatTime(position)} label='Time' />
    </div>
  )
}

export default TransportCounter
