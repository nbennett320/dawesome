import React from 'react'
import { dbToPosition } from '../../types/mixer'
import { RegisterMeter } from './useMixerMeters'

interface Props {
  meterKey: string
  register: RegisterMeter
  // the space above and below the scale, to line up with a fader's travel
  inset: number
}

// a level meter: green up to -10 dB, then yellow, then red above 0 dB.
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
    <div className='relative w-2 h-full' aria-hidden>
      <div className='absolute left-0 right-0 rounded-sm bg-gray-800' style={{ top: inset, bottom: inset }}>
        <div
          ref={bar}
          className='absolute inset-0 rounded-sm'
          style={{
            clipPath: 'inset(100% 0 0 0)',
            background: `linear-gradient(to top, #22c55e 0%, #22c55e ${yellow}, #eab308 ${yellow}, #eab308 ${red}, #ef4444 ${red})`,
          }}
        />
        <div ref={hold} className='absolute left-0 right-0 h-px bg-white opacity-0' style={{ bottom: 0 }} />
      </div>
    </div>
  )
}

export default Meter
