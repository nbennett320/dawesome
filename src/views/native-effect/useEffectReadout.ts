import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { EffectReadout } from '../../types/nativeEffect'
import { POLL_MS } from './spectrum'
import { useVisible } from './useEffectSpectrum'

// a readout and when it came (performance.now(), ms): a panel runs an LFO
// on from there between polls, so its picture moves at the display's rate
export interface Reading {
  readout: EffectReadout
  at: number
  // it's being run: frames went up since the last reading. a resting or
  // bypassed effect's picture holds still
  running: boolean
}

// what a built-in effect is doing, polled while `active` (its tab showing)
// and the window can be seen, as the spectrum is (useEffectSpectrum). each
// reading goes to `onReading`, not into state, so the panel isn't drawn
// again by React each poll. a poll that fails is skipped quietly
export const useEffectReadout = (
  slotId: number,
  active: boolean,
  onReading: (reading: Reading) => void,
): void => {
  const handler = React.useRef(onReading)
  handler.current = onReading
  const visible = useVisible()

  React.useEffect(() => {
    if(!active || !visible) return undefined
    let live = true
    let asking = false
    let last: number | null = null
    const poll = () => {
      if(asking) return
      asking = true
      invoke<EffectReadout>('get_effect_readout', { slotId })
        .then(readout => {
          if(!live) return
          const running = last !== null && readout.written > last
          last = readout.written
          handler.current({ readout, at: performance.now(), running })
        })
        .catch(() => {})
        .finally(() => { asking = false })
    }
    poll()
    const timer = setInterval(poll, POLL_MS)
    return () => {
      live = false
      clearInterval(timer)
    }
  }, [slotId, active, visible])
}
