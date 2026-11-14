// a track's right-click menu, from its LED in the mixer or the playlist. it
// arms the track to record, or swaps the menu for the tray of colors. drawn
// over the whole window, at the click, so it isn't cut off by the strip or
// the canvas it was opened from
import React from 'react'
import { createPortal } from 'react-dom'
import ClipMenu from '../playlist/ClipMenu'
import { TrackColorPalette } from './TrackColorPalette'

export interface TrackMenuAt {
  // where it opens, in the window
  clientX: number
  clientY: number
}

// room the tray needs from the window's edges (it's about 96 × 52)
const TRAY_ROOM = { width: 104, height: 60 }

const TrackMenu = ({ at, label, color, onChange, arm, onClose }: {
  at: TrackMenuAt
  // the track's name
  label: string
  color: number
  onChange: (color: number) => void
  // arming the track to record (not for the master): whether it is armed, and
  // arming or disarming it
  arm?: { armed: boolean, onToggle: () => void }
  onClose: () => void
}) => {
  const [step, setStep] = React.useState<'menu' | 'colors'>('menu')
  // a choice closes the menu and then picks (see ClipMenu), so the close
  // waits a moment, and is dropped if a pick followed it
  const picked = React.useRef(false)
  const closeMenu = React.useCallback(() => {
    queueMicrotask(() => {
      if(!picked.current) onClose()
    })
  }, [onClose])
  const pickColor = () => {
    picked.current = true
    setStep('colors')
  }

  const x = Math.min(at.clientX, window.innerWidth - TRAY_ROOM.width)
  const y = Math.min(at.clientY, window.innerHeight - TRAY_ROOM.height)

  return createPortal(
    <div className='fixed inset-0 z-50 pointer-events-none [&>*]:pointer-events-auto'>
      {step === 'menu' ? (
        <ClipMenu
          x={at.clientX}
          y={at.clientY}
          items={[
            ...(arm ? [{ label: arm.armed ? 'Disarm recording' : 'Arm for recording', onSelect: arm.onToggle }] : []),
            { label: 'Change channel color', onSelect: pickColor },
          ]}
          onClose={closeMenu}
        />
      ) : (
        <TrackColorPalette
          label={label}
          color={color}
          onChange={onChange}
          onClose={onClose}
          className='absolute'
          style={{ left: x, top: y }}
        />
      )}
    </div>,
    document.body,
  )
}

export default TrackMenu
