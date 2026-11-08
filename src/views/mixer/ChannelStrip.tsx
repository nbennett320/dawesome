import React from 'react'
import { formatDb, formatPan, MAX_DB, MAX_NAME_CHARS, MIN_DB } from '../../types/mixer'
import Fader, { THUMB_HEIGHT } from './Fader'
import Meter from './Meter'
import { RegisterMeter } from './useMixerMeters'

// the volume in dB, typed in. "-inf" or "-∞" is silent
const parseDb = (typed: string): number | null => {
  const text = typed.trim().toLowerCase()
  if(text === '-inf' || text === '-∞' || text === 'inf') return MIN_DB

  const db = parseFloat(text)
  return Number.isFinite(db) ? Math.min(MAX_DB, Math.max(MIN_DB, db)) : null
}

// the volume's readout, which can be typed into
const DbField = ({ label, value, onChange }: {
  label: string
  value: number
  onChange: (db: number) => void
}) => {
  const [draft, setDraft] = React.useState<string | null>(null)

  const commit = () => {
    if(draft === null) return
    const db = parseDb(draft)
    if(db !== null) onChange(db)
    setDraft(null)
  }

  return (
    <input
      type='text'
      inputMode='decimal'
      aria-label={`${label} volume in dB`}
      title='Volume in dB'
      className='w-full text-center text-xs tabular-nums text-gray-700 rounded border border-gray-300 px-1'
      value={draft ?? formatDb(value)}
      onFocus={e => e.currentTarget.select()}
      onChange={e => setDraft(e.currentTarget.value)}
      onBlur={commit}
      onKeyDown={e => {
        if(e.key === 'Enter') e.currentTarget.blur()
        if(e.key === 'Escape') setDraft(null)
      }}
    />
  )
}

// the strip's name. double-click to rename it, if it can be renamed
const NameLabel = ({ label, silenced, onRename }: {
  label: string
  silenced: boolean
  onRename?: (name: string) => void
}) => {
  const [draft, setDraft] = React.useState<string | null>(null)

  if(draft !== null && onRename) {
    const commit = () => {
      onRename(draft)
      setDraft(null)
    }

    return (
      <input
        type='text'
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        aria-label='Track name'
        placeholder='Track name'
        maxLength={MAX_NAME_CHARS}
        className='w-full text-xs text-center rounded border border-sky-400 px-1'
        value={draft}
        onFocus={e => e.currentTarget.select()}
        onChange={e => setDraft(e.currentTarget.value)}
        onBlur={commit}
        onKeyDown={e => {
          if(e.key === 'Enter') commit()
          if(e.key === 'Escape') setDraft(null)
        }}
      />
    )
  }

  let title = label
  if(silenced) title += ' (silenced by solo)'
  if(onRename) title += '. Double-click to rename'

  return (
    <div
      className={`text-xs font-medium truncate text-center ${silenced ? 'text-gray-400 italic' : 'text-gray-800'}`}
      title={title}
      onDoubleClick={() => { if(onRename) setDraft(label) }}
    >
      {label}
    </div>
  )
}

const ToggleButton = ({ label, title, on, onClass, onClick }: {
  label: string
  title: string
  on: boolean
  // colours while it's on
  onClass: string
  onClick: () => void
}) => (
  <button
    type='button'
    title={title}
    aria-pressed={on}
    className={`flex-1 text-xs font-semibold rounded border py-0.5 ${
      on ? onClass : 'border-gray-300 text-gray-500 hover:bg-slate-100'
    }`}
    onClick={onClick}
  >
    {label}
  </button>
)

interface Props {
  label: string
  volumeDb: number
  onVolume: (db: number) => void
  meterKey: string
  register: RegisterMeter
  // the pan control (track strips)
  pan?: { value: number, onChange: (pan: number) => void }
  // mute and solo (track strips)
  mute?: { on: boolean, onChange: (on: boolean) => void }
  solo?: { on: boolean, onChange: (on: boolean) => void }
  // silenced by another track's solo
  silenced?: boolean
  // renaming (track strips). an empty name goes back to the default
  onRename?: (name: string) => void
  // under the fader, in place of mute and solo (the master strip)
  footer?: React.ReactNode
  // above the fader, in place of the pan control (the master strip)
  header?: React.ReactNode
  wide?: boolean
}

// one column of the mixer: name, pan, fader and meter, volume, mute and solo
const ChannelStrip = ({
  label,
  volumeDb,
  onVolume,
  meterKey,
  register,
  pan,
  mute,
  solo,
  silenced = false,
  onRename,
  footer,
  header,
  wide = false,
}: Props) => (
  <div
    className={`flex flex-col gap-2 h-full p-2 rounded border border-gray-200 bg-white ${wide ? 'w-28' : 'w-24'}`}
    aria-label={label}
    role='group'
  >
    <div className='h-5'>
      <NameLabel label={label} silenced={silenced} onRename={onRename} />
    </div>

    <div className='flex flex-col gap-0.5 h-9'>
      {pan && (
        <>
          <div className='flex flex-row justify-between text-[10px] text-gray-500 select-none'>
            <span>Pan</span>
            <span className='tabular-nums'>{formatPan(pan.value)}</span>
          </div>
          <input
            type='range'
            aria-label={`${label} pan`}
            title='Double-click to centre'
            className='w-full'
            min={-1}
            max={1}
            step={0.01}
            value={pan.value}
            onChange={e => pan.onChange(parseFloat(e.currentTarget.value))}
            onDoubleClick={() => pan.onChange(0)}
          />
        </>
      )}
      {!pan && header}
    </div>

    <div className='flex-1 min-h-0 flex flex-row justify-center'>
      <Fader label={`${label} volume`} value={volumeDb} onChange={onVolume} onReset={() => onVolume(0)}>
        <Meter meterKey={meterKey} register={register} inset={THUMB_HEIGHT / 2} />
      </Fader>
    </div>

    <DbField label={label} value={volumeDb} onChange={onVolume} />

    <div className='flex flex-row gap-1 h-6'>
      {mute && (
        <ToggleButton
          label='M'
          title='Mute'
          on={mute.on}
          onClass='bg-amber-400 border-amber-500 text-amber-950'
          onClick={() => mute.onChange(!mute.on)}
        />
      )}
      {solo && (
        <ToggleButton
          label='S'
          title='Solo: only hear soloed tracks'
          on={solo.on}
          onClass='bg-lime-400 border-lime-500 text-lime-950'
          onClick={() => solo.onChange(!solo.on)}
        />
      )}
      {!mute && !solo && footer}
    </div>
  </div>
)

export default ChannelStrip
