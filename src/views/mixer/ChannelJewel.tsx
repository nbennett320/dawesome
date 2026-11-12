// a track's LED jewel, beside its name on its mixer strip: unlit in the
// track's color while it's quiet, lit while it's sounding (useMixerMeters
// lights it)
import React from 'react'
import { useThemeChange } from '../../theme'
import { Led } from '../../components/fourtrack'
import { ledTint, trackColorCss } from './TrackColorPalette'
import TrackColorMenu, { TrackMenuAt } from './TrackColorMenu'

// a track's LED jewel. a click mutes the track, a shift+click solos it (as
// its lip's LED does in the playlist), and a right-click opens its menu
const ChannelJewel = ({
  label, color, onChange, muted, soloed, onMute, onSolo, ledRef,
}: {
  // the track's name, for screen readers and tooltips
  label: string
  // its color (see TRACK_COLORS)
  color: number
  onChange: (color: number) => void
  muted: boolean
  soloed: boolean
  onMute: (mute: boolean) => void
  onSolo: (solo: boolean) => void
  // the LED, for useMixerMeters to light while the track sounds
  ledRef?: (led: HTMLElement | null) => void
}) => {
  useThemeChange()
  const [menu, setMenu] = React.useState<TrackMenuAt | null>(null)
  const close = React.useCallback(() => setMenu(null), [])

  return (
    <div className='relative flex-none flex items-center'>
      <button
        type='button'
        className='ft-jewel'
        aria-label={`${label}: click to mute, shift+click to solo`}
        aria-pressed={muted}
        aria-haspopup='menu'
        title='Click to mute, Shift+click to solo. Right-click to change the color'
        onClick={e => {
          if(e.shiftKey) onSolo(!soloed)
          else onMute(!muted)
        }}
        onContextMenu={e => {
          e.preventDefault()
          setMenu({ clientX: e.clientX, clientY: e.clientY })
        }}
      >
        <Led ref={ledRef} style={ledTint(trackColorCss(color))} />
      </button>
      {menu && <TrackColorMenu at={menu} label={label} color={color} onChange={onChange} onClose={close} />}
    </div>
  )
}

export default ChannelJewel
