import React from 'react'
import { ROW_VOLUME_MAX } from '../../types/sequencer'
import { formatPan } from '../../types/mixer'
import { AnalogButton, Knob, Led } from '../../components/fourtrack'
import { NameInput } from '../../components/common'

// the longest a row's name can be (as the engine keeps it)
const MAX_NAME_CHARS = 40

// how long a click on the name waits to see whether it's a double-click
// (which renames) before doing what a click does
const CLICK_WAIT_MS = 220

interface Props {
  name: string
  // the name's tooltip, saying what clicking it does
  nameTitle: string
  onName: () => void
  // greyed, and the name can't be clicked (a synth that isn't installed)
  disabled?: boolean
  // said after the name, faintly ("Not installed", or the sample's name)
  note?: string
  // renamed by double-clicking the name: what the field starts with, what
  // it shows when empty, and the new name (blank to clear it). left out,
  // the name can't be renamed
  rename?: {
    initial: string
    placeholder: string
    onRename: (name: string) => void
  }
  muted: boolean
  onMute: (muted: boolean) => void
  volume: number
  onVolume: (volume: number) => void
  // left out, there's no pan knob
  pan?: number
  onPan?: (pan: number) => void
  // what the remove key's tooltip says it takes away
  removeTitle: string
  onRemove: () => void
}

// a volume as a percentage of the sample's (or synth's) own level
const percent = (volume: number) => `${Math.round(volume * 100)}%`

// a small knob with its caption under it
const CaptionedKnob = ({ caption, ...knob }: React.ComponentProps<typeof Knob> & { caption: string }) => (
  <div className='flex flex-col items-center gap-0.5'>
    <Knob size={24} {...knob} />
    <span className='ft-label text-[8px] leading-none tracking-normal'>{caption}</span>
  </div>
)

// the left end of a sequencer's row, the same for a sample and a synth: its
// name, mute, volume, pan (samples) and, set apart, remove
const RowControls = ({
  name, nameTitle, onName, disabled = false, note, rename, muted, onMute, volume, onVolume, pan, onPan,
  removeTitle, onRemove,
}: Props) => {
  const [renaming, setRenaming] = React.useState(false)
  const clickTimer = React.useRef<ReturnType<typeof setTimeout> | null>(null)

  const cancelClick = () => {
    if(clickTimer.current) clearTimeout(clickTimer.current)
    clickTimer.current = null
  }
  React.useEffect(() => cancelClick, [])

  return (
    <div className={`ft-strip shrink-0 w-80 h-11 flex flex-row items-center px-1.5 ${disabled ? 'opacity-60' : ''}`}>
      {renaming && rename ? (
        <NameInput
          initial={rename.initial}
          aria-label='Row name'
          placeholder={rename.placeholder}
          maxLength={MAX_NAME_CHARS}
          className='flex-1 min-w-0 h-7 mr-3'
          onDone={text => {
            if(text !== null) rename.onRename(text)
            setRenaming(false)
          }}
        />
      ) : (
        <button
          type='button'
          disabled={disabled}
          className={`ft-scribble flex-1 min-w-0 mr-3 flex flex-row items-baseline gap-1.5 text-left text-xs font-medium px-1 py-0.5 enabled:hover:bg-surface-alt ${muted || disabled ? 'text-ink-faint italic' : ''}`}
          title={nameTitle}
          onClick={ev => {
            if(!rename) {
              onName()
              return
            }
            // the second click of a double-click: that renames instead
            if(ev.detail > 1) return
            cancelClick()
            clickTimer.current = setTimeout(() => {
              clickTimer.current = null
              onName()
            }, CLICK_WAIT_MS)
          }}
          onDoubleClick={() => {
            if(!rename) return
            cancelClick()
            setRenaming(true)
          }}
        >
          <span className='truncate'>{name}</span>
          {note && <span className='truncate text-[10px] font-normal text-ink-faint shrink-[2]'>{note}</span>}
        </button>
      )}

      <div className='flex flex-row items-center gap-3'>
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
        <CaptionedKnob
          caption='Vol'
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
        {pan !== undefined && onPan && (
          <CaptionedKnob
            caption='Pan'
            min={-1}
            max={1}
            center={0}
            step={0.01}
            value={pan}
            onChange={onPan}
            label={`${name} pan`}
            formatValue={formatPan}
            title={`Pan ${formatPan(pan)}. Double-click to centre`}
          />
        )}
      </div>

      <div className='self-stretch w-px my-2 mx-2.5 bg-line-strong' aria-hidden='true' />
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
}

export default RowControls
