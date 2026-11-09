import React from 'react'
import { formatDb, formatPan, MAX_DB, MAX_NAME_CHARS, MIN_DB } from '../../types/mixer'
import { Knob, MuteSolo } from '../../components/fourtrack'
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
      className='ft-readout'
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
        className='ft-scribble w-full h-full text-xs text-center px-1 outline-none ring-2 ring-sky-400'
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
      className={`ft-scribble h-full leading-5 px-1 text-xs font-medium truncate text-center ${silenced ? 'text-slate-400 italic' : ''}`}
      title={title}
      onDoubleClick={() => { if(onRename) setDraft(label) }}
    >
      {label}
    </div>
  )
}

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
    className={`ft-strip flex flex-col items-stretch gap-2 h-full p-2 ${wide ? 'w-28' : 'w-24'}`}
    aria-label={label}
    role='group'
  >
    <div className='h-5'>
      <NameLabel label={label} silenced={silenced} onRename={onRename} />
    </div>

    <div className='flex flex-col items-center h-16'>
      {pan && (
        <>
          <div className='w-full flex flex-row justify-between items-baseline'>
            <span className='ft-label'>Pan</span>
            <span className='ft-label tabular-nums tracking-normal'>{formatPan(pan.value)}</span>
          </div>
          <div className='flex flex-row items-center gap-1.5 mt-1'>
            <span className='ft-label text-[8px] tracking-normal'>L</span>
            <Knob
              label={`${label} pan`}
              title='Drag up or down, or double-click to centre'
              size={38}
              min={-1}
              max={1}
              center={0}
              dragRange={150}
              value={pan.value}
              onChange={pan.onChange}
              formatValue={formatPan}
            />
            <span className='ft-label text-[8px] tracking-normal'>R</span>
          </div>
        </>
      )}
      {!pan && header}
    </div>

    <div className='flex-1 min-h-0 flex flex-row justify-center'>
      <Fader label={`${label} volume`} value={volumeDb} onChange={onVolume} onReset={() => onVolume(0)}>
        <Meter meterKey={meterKey} register={register} inset={THUMB_HEIGHT / 2 - 4} />
      </Fader>
    </div>

    <DbField label={label} value={volumeDb} onChange={onVolume} />

    <div className='flex flex-row justify-center h-11'>
      {mute && solo && (
        <MuteSolo
          track={label}
          size='sm'
          direction='row'
          muted={mute.on}
          soloed={solo.on}
          implied={silenced}
          onMutedChange={mute.onChange}
          onSoloedChange={solo.onChange}
        />
      )}
      {!(mute && solo) && footer}
    </div>
  </div>
)

export default ChannelStrip
