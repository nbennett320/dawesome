import React from 'react'
import { AnalogButton, Led } from '../../components/fourtrack'
import { fxLabel } from '../../types/effects'
import { KeysIcon } from '../../components/toolbars/browser/icons'

interface Props {
  // the strip's name, for screen readers
  channel: string
  // how many effects the channel has
  count: number
  // at least one of them is on
  lit: boolean
  // the track has an instrument (counted with the effects)
  instrument?: boolean
  onOpen: () => void
}

// what the button says it opens
const fxTitle = (count: number, lit: boolean, instrument: boolean): string => {
  if(!count) return 'Open the Effects Rack. Drag an effect from the sidebar onto the strip to add one'
  const effects = count - (instrument ? 1 : 0)
  const what = [
    instrument ? 'An instrument' : '',
    effects ? `${effects} effect${effects === 1 ? '' : 's'}` : '',
  ].filter(Boolean).join(' and ')
  return `${what}${lit ? '' : ', all off'}. Open ${count === 1 ? 'it' : 'them'} in the Effects Rack`
}

// a strip's effects: "FX 3", with a light like the topbar's toggles that's
// lit while any of them is on, and a few keys when the track has an
// instrument. opens the Effects Rack on the channel
const FxButton = ({ channel, count, lit, instrument = false, onOpen }: Props) => (
  <div className='flex flex-row items-center gap-1'>
    <Led color='blue' state={lit ? 'on' : 'off'} />
    <AnalogButton
      variant='text'
      size='xs'
      className='flex-1'
      aria-label={`${channel} effects`}
      title={fxTitle(count, lit, instrument)}
      onPress={onOpen}
    >
      <span className='inline-flex items-center gap-1'>
        {instrument && <KeysIcon className='w-3 h-3' />}
        {fxLabel(count)}
      </span>
    </AnalogButton>
  </div>
)

export default FxButton
