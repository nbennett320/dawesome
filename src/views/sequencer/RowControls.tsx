import React from 'react'
import { ROW_VOLUME_MAX } from '../../types/sequencer'
import { AnalogButton, Knob, Led } from '../../components/fourtrack'

interface Props {
  name: string
  // the name's tooltip, saying what clicking it does
  nameTitle: string
  onName: () => void
  // greyed, and the name can't be clicked (a synth that isn't installed)
  disabled?: boolean
  // said after the name, faintly ("Not installed")
  note?: string
  muted: boolean
  onMute: (muted: boolean) => void
  volume: number
  onVolume: (volume: number) => void
  // what the remove key's tooltip says it takes away
  removeTitle: string
  onRemove: () => void
}

// a volume as a percentage of the sample's (or synth's) own level
const percent = (volume: number) => `${Math.round(volume * 100)}%`

// the left end of a sequencer's row, the same for a sample and a synth: its
// name, mute, volume and remove
const RowControls = ({
  name, nameTitle, onName, disabled = false, note, muted, onMute, volume, onVolume, removeTitle, onRemove,
}: Props) => (
  <div className={`ft-strip shrink-0 w-64 h-11 flex flex-row items-center gap-1.5 px-1.5 ${disabled ? 'opacity-60' : ''}`}>
    <button
      type='button'
      disabled={disabled}
      className={`ft-scribble flex-1 min-w-0 flex flex-row items-baseline gap-1.5 text-left text-xs font-medium px-1 py-0.5 enabled:hover:bg-surface-alt ${muted || disabled ? 'text-ink-faint italic' : ''}`}
      title={nameTitle}
      onClick={onName}
    >
      <span className='truncate'>{name}</span>
      {note && <span className='truncate text-[10px] font-normal text-ink-faint shrink-[2]'>{note}</span>}
    </button>
    <div className='flex flex-col items-center gap-0.5'>
      <Led color='amber' state={muted ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='xs'
        latched={muted}
        aria-label={`Mute ${name}`}
        title='Mute'
        onPress={() => onMute(!muted)}
      >
        M
      </AnalogButton>
    </div>
    <Knob
      size={26}
      min={0}
      max={ROW_VOLUME_MAX}
      center={1}
      step={0.01}
      value={volume}
      onChange={onVolume}
      label={`${name} volume`}
      formatValue={percent}
      title={`Volume ${percent(volume)}. Double-click for 100%`}
    />
    <AnalogButton
      variant='text'
      size='xs'
      aria-label={`Remove ${name}`}
      title={removeTitle}
      onPress={onRemove}
    >
      ×
    </AnalogButton>
  </div>
)

export default RowControls
