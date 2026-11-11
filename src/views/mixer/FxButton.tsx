import React from 'react'
import { AnalogButton, Led } from '../../components/fourtrack'
import { fxLabel } from '../../types/effects'

interface Props {
  // the strip's name, for screen readers
  channel: string
  // how many effects the channel has
  count: number
  // at least one of them is on
  lit: boolean
  onOpen: () => void
}

// a strip's effects: "FX 3", with a light like the topbar's toggles that's
// lit while any of them is on. opens the Effects Rack on the channel
const FxButton = ({ channel, count, lit, onOpen }: Props) => (
  <div className='flex flex-row items-center gap-1'>
    <Led color='blue' state={lit ? 'on' : 'off'} />
    <AnalogButton
      variant='text'
      size='xs'
      className='flex-1'
      aria-label={`${channel} effects`}
      title={count
        ? `${count} effect${count === 1 ? '' : 's'}${lit ? '' : ', all off'}. Open them in the Effects Rack`
        : 'Open the Effects Rack. Drag an effect from the sidebar onto the strip to add one'}
      onPress={onOpen}
    >
      {fxLabel(count)}
    </AnalogButton>
  </div>
)

export default FxButton
