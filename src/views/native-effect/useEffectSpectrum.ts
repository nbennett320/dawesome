import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { EffectSpectrum } from '../../types/nativeEffect'
import { follow, Picture, POLL_MS } from './spectrum'

// whether the app's window can be seen (not minimized or hidden)
const useVisible = (): boolean => {
  const [visible, setVisible] = React.useState(() => document.visibilityState !== 'hidden')
  React.useEffect(() => {
    const update = () => setVisible(document.visibilityState !== 'hidden')
    document.addEventListener('visibilitychange', update)
    return () => document.removeEventListener('visibilitychange', update)
  }, [])
  return visible
}

// a built-in effect's output, as an analyzer's picture (spectrum.ts), polled
// while `active` (its tab showing) and the window can be seen. each new
// picture goes to `onFrame`, not into state, so the panel isn't drawn again
// by React each poll. a poll that fails (the effect gone, an undo under way)
// is skipped quietly
export const useEffectSpectrum = (
  slotId: number,
  active: boolean,
  onFrame: (picture: Picture | null) => void,
): void => {
  const picture = React.useRef<Picture | null>(null)
  const frame = React.useRef(onFrame)
  frame.current = onFrame
  const visible = useVisible()

  React.useEffect(() => {
    if(!active || !visible) return undefined
    let live = true
    let asking = false
    const poll = () => {
      // one at a time
      if(asking) return
      asking = true
      invoke<EffectSpectrum | null>('get_effect_spectrum', { slotId })
        .then(got => {
          if(!live) return
          picture.current = follow(picture.current, got)
          frame.current(picture.current)
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
