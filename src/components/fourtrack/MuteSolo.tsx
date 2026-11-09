import React from 'react'
import { AnalogButton, AnalogButtonSize } from './AnalogButton'
import { Led, LedState } from './Led'
import { cx } from './util'

export interface MuteSoloProps {
  // the track's name, for labels, e.g. "Track 1"
  track: string
  muted: boolean
  soloed: boolean
  // silenced because another track is soloed: the mute LED glows dimly
  implied?: boolean
  onMutedChange: (muted: boolean) => void
  onSoloedChange: (soloed: boolean) => void
  size?: AnalogButtonSize
  // side by side, or stacked
  direction?: 'row' | 'column'
}

const muteLed = (muted: boolean, implied?: boolean): LedState => {
  if(muted) return 'on'
  return implied ? 'dim' : 'off'
}

// mute (amber) and solo (lime) keys, each with an LED
export const MuteSolo = ({
  track,
  muted,
  soloed,
  implied,
  onMutedChange,
  onSoloedChange,
  size = 'md',
  direction = 'column',
}: MuteSoloProps) => (
  <div className={cx('ft-mutesolo', direction === 'row' && 'ft-mutesolo--row')}>
    <div className='ft-mutesolo__key'>
      <Led color='amber' state={muteLed(muted, implied)} />
      <AnalogButton
        variant='text'
        size={size}
        latched={muted}
        aria-label={`Mute ${track}`}
        title={implied ? 'Mute (silenced by another track\'s solo)' : 'Mute'}
        onPress={() => onMutedChange(!muted)}
      >
        M
      </AnalogButton>
    </div>
    <div className='ft-mutesolo__key'>
      <Led color='lime' state={soloed ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size={size}
        latched={soloed}
        aria-label={`Solo ${track}`}
        title='Solo: only hear soloed tracks'
        onPress={() => onSoloedChange(!soloed)}
      >
        S
      </AnalogButton>
    </div>
  </div>
)
