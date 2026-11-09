import React from 'react'
import { dbToPosition } from '../../types/mixer'
import { RegisterMeter } from './useMixerMeters'

interface Props {
  meterKey: string
  register: RegisterMeter
  // the space above and below the scale, to line up with a fader's travel
  inset: number
}

// a segmented level meter: lime up to -10 dB, then amber, then red above 0 dB.
// useMixerMeters draws into it
const Meter = ({ meterKey, register, inset }: Props) => {
  const bar = React.useRef<HTMLDivElement>(null)
  const hold = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    if(!bar.current || !hold.current) return undefined

    register(meterKey, { bar: bar.current, hold: hold.current })
    return () => register(meterKey, null)
  }, [meterKey, register])

  const yellow = `${dbToPosition(-10) * 100}%`
  const red = `${dbToPosition(0) * 100}%`

  return (
    <div className='relative w-2.5 h-full' aria-hidden>
      <div className='ft-meter-glass absolute left-0 right-0 overflow-hidden' style={{ top: inset, bottom: inset }}>
        <div
          ref={bar}
          className='absolute inset-0 rounded-sm'
          style={{
            clipPath: 'inset(100% 0 0 0)',
            background: `linear-gradient(to top, #84cc16 0%, #84cc16 ${yellow}, #f59e0b ${yellow}, #f59e0b ${red}, #ef4444 ${red})`,
          }}
        />
        <div ref={hold} className='absolute left-0 right-0 h-px bg-slate-600 opacity-0' style={{ bottom: 0 }} />
        {/* gaps between the LED segments */}
        <div className='ft-meter-glass__segments' />
      </div>
    </div>
  )
}

export default Meter
