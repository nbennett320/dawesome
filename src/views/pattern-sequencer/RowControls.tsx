import React from 'react'
import { createPortal } from 'react-dom'
import { ROW_VOLUME_MAX } from '../../types/sequencer'
import { formatPan } from '../../types/mixer'
import { AnalogButton, Knob, Led } from '../../components/fourtrack'
import { NameInput } from '../../components/common'
import { CloseIcon, PanIcon, VolumeIcon } from '../../components/icons'
import ClipMenu from '../playlist/ClipMenu'

// a field in place of the name, opened from the right-click menu
export interface RowField {
  // read out
  label: string
  // what it starts with, and shows when empty
  initial: string
  placeholder: string
  maxLength: number
  // what was typed, on Enter or when the field is left (on Escape, nothing)
  onDone: (text: string) => void
}

// an item of the right-click menu: one that does something, or one that
// opens a field in place of the name (Rename…, a pitch)
export type RowMenuItem = { label: string, title?: string } & (
  | { onSelect: () => void }
  | { field: RowField }
)

interface Props {
  name: string
  // the name's tooltip, saying what clicking it does
  nameTitle: string
  onName: () => void
  // greyed, and the name can't be clicked (a synth that isn't installed)
  disabled?: boolean
  // said after the name, faintly ("Not installed", or the sample's name)
  note?: string
  // the right-click menu, before Remove
  menu: RowMenuItem[]
  muted: boolean
  onMute: (muted: boolean) => void
  volume: number
  onVolume: (volume: number) => void
  pan: number
  onPan: (pan: number) => void
  // pressed on the grip at the left, to drag the row up or down
  onGripDown: (ev: React.MouseEvent) => void
  // what the remove key's tooltip says it takes away
  removeTitle: string
  onRemove: () => void
}

// a volume as a percentage of the sample's (or synth's) own level
const percent = (volume: number) => `${Math.round(volume * 100)}%`

// the left end of a row of the Pattern Sequencer, the same for a sample and
// a synth: its grip, name, mute, volume, pan and, set apart, remove.
// right-click it for a menu (`menu`, then Remove)
const RowControls = ({
  name, nameTitle, onName, disabled = false, note, menu: items, muted, onMute, volume, onVolume, pan, onPan,
  onGripDown, removeTitle, onRemove,
}: Props) => {
  // the field open in place of the name
  const [field, setField] = React.useState<RowField | null>(null)
  // where the right-click menu is open, in the window
  const [menu, setMenu] = React.useState<{ x: number, y: number } | null>(null)
  const closeMenu = React.useCallback(() => setMenu(null), [])

  return (
    <div
      className={`ft-strip ft-strip--module shrink-0 w-80 h-11 flex flex-row items-center px-2 ${disabled ? 'opacity-60' : ''}`}
      onContextMenu={ev => {
        ev.preventDefault()
        setMenu({ x: ev.clientX, y: ev.clientY })
      }}
    >
      <div
        className='shrink-0 self-stretch flex items-center px-0.5 mr-1 text-ink-faint hover:text-ink-soft cursor-grab leading-none tracking-[-0.2em] text-sm'
        title='Drag to move the row'
        aria-hidden='true'
        onMouseDown={onGripDown}
      >
        ⋮⋮
      </div>
      {field ? (
        <NameInput
          initial={field.initial}
          aria-label={field.label}
          placeholder={field.placeholder}
          maxLength={field.maxLength}
          className='flex-1 min-w-0 h-7 mr-2'
          onDone={text => {
            if(text !== null) field.onDone(text)
            setField(null)
          }}
        />
      ) : (
        <button
          type='button'
          disabled={disabled}
          className={`ft-scribble ft-scribble--name flex-1 min-w-0 mr-2 flex flex-row items-baseline gap-1.5 text-left px-1 py-1 rounded-sm enabled:hover:bg-surface-alt ${muted || disabled ? 'ft-scribble--quiet' : ''}`}
          title={nameTitle}
          onClick={onName}
        >
          <span className='truncate'>{name}</span>
          {note && <span className='truncate normal-case tracking-normal font-normal text-ink-faint shrink-[2]'>{note}</span>}
        </button>
      )}

      <div className='ft-well shrink-0 flex flex-row items-center gap-2.5 px-2 py-1'>
        <div className='flex flex-row items-center gap-1'>
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
          size={24}
          icon={<VolumeIcon />}
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
        <Knob
          size={24}
          icon={<PanIcon />}
          min={-1}
          max={1}
          center={0}
          step={0.01}
          value={pan}
          onChange={onPan}
          label={`${name} pan`}
          formatValue={formatPan}
          title={`Pan ${formatPan(pan)}. Double-click to center`}
        />
      </div>

      <div className='self-stretch w-px my-2.5 mx-2 bg-line' aria-hidden='true' />
      <AnalogButton
        variant='text'
        size='xs'
        aria-label={`Remove ${name}`}
        title={removeTitle}
        onPress={onRemove}
      >
        {/* an icon, not "×": a key's letter-spacing put the character off
            center */}
        <CloseIcon width={10} height={10} />
      </AnalogButton>

      {menu && createPortal(
        // over the whole window, so the sequencer's scrolling box doesn't
        // cut it off
        <div className='fixed inset-0 z-50 pointer-events-none [&>*]:pointer-events-auto'>
          <ClipMenu
            x={menu.x}
            y={menu.y}
            items={[
              ...items.map(item => ({
                label: item.label,
                title: item.title,
                onSelect: 'field' in item ? () => setField(item.field) : item.onSelect,
              })),
              'divider',
              { label: 'Remove', title: removeTitle, danger: true, onSelect: onRemove },
            ]}
            onClose={closeMenu}
          />
        </div>,
        document.body,
      )}
    </div>
  )
}

export default RowControls
