import React from 'react'
import { formatDb, formatPan, MAX_DB, MAX_NAME_CHARS, MIN_DB } from '../../types/mixer'
import { AnalogButton, Knob, Led, MuteSolo } from '../../components/fourtrack'
import {
  InputChoice, defaultInput, fitInput, inputChoices, inputKey, inputLabel, parseInputKey,
} from '../../types/audio'
import Fader, { THUMB_HEIGHT } from './Fader'
import Meter from './Meter'
import FxJack from './FxJack'
import { DROP_CHANNEL } from './useChannelDrop'
import { RegisterMeter } from './useMixerMeters'
import { clamp } from '../../util/helpers'
import { NameInput, ReadoutInput } from '../../components/common'
import ChannelJewel from './ChannelJewel'

// the volume in dB, typed in. "-inf" or "-∞" is silent
const parseDb = (typed: string): number | null => {
  const text = typed.trim().toLowerCase()
  if(text === '-inf' || text === '-∞' || text === 'inf') return MIN_DB

  const db = parseFloat(text)
  return Number.isFinite(db) ? clamp(db, MIN_DB, MAX_DB) : null
}

// the volume's readout, which can be typed into
const DbField = ({ label, value, onChange }: {
  label: string
  value: number
  onChange: (db: number) => void
}) => (
  <ReadoutInput
    aria-label={`${label} volume in dB`}
    title='Volume in dB'
    value={formatDb(value)}
    onCommit={typed => {
      const db = parseDb(typed)
      if(db !== null) onChange(db)
    }}
  />
)

// the strip's name. double-click to rename it, if it can be renamed
const NameLabel = ({ label, muted, silenced, onRename }: {
  label: string
  muted: boolean
  silenced: boolean
  onRename?: (name: string) => void
}) => {
  const [renaming, setRenaming] = React.useState(false)

  if(renaming && onRename) {
    return (
      <NameInput
        initial={label}
        aria-label='Track name'
        placeholder='Track name'
        maxLength={MAX_NAME_CHARS}
        className='w-full h-full text-center'
        onDone={name => {
          if(name !== null) onRename(name)
          setRenaming(false)
        }}
      />
    )
  }

  let title = label
  if(muted) title += ' (muted)'
  else if(silenced) title += ' (silenced by solo)'
  if(onRename) title += '. Double-click to rename'

  return (
    <div
      className={`ft-scribble ft-scribble--name h-full leading-5 px-1 truncate text-center ${
        muted || silenced ? 'ft-scribble--quiet' : ''} ${silenced ? 'italic' : ''}`}
      title={title}
      onDoubleClick={() => { if(onRename) setRenaming(true) }}
    >
      {label}
    </div>
  )
}

// arming a track to record, and the input channels it records
export interface RecordControl {
  // the input channels it records, when it's armed
  armed: InputChoice | null
  // the input's channels to choose from (null without an input)
  channels: number | null
  // recording now (an armed track's LED flashes)
  recording: boolean
  onChange: (input: InputChoice | null) => void
}

const RecordArm = ({ label, record }: { label: string, record: RecordControl }) => {
  const { armed, channels, recording, onChange } = record
  // what it was set to record: what it was armed with, or the first two
  // channels (or the only one)
  const [choice, setChoice] = React.useState<InputChoice>(armed ?? defaultInput(channels))
  React.useEffect(() => { if(armed) setChoice(armed) }, [armed])
  // and what it will record, on this input. the input's channels can be
  // learnt after this is shown, or change with the input, or the project
  // may have been armed with another input
  const shown = fitInput(choice, channels)

  let led: 'off' | 'on' | 'blink' = 'off'
  if(armed) led = recording ? 'blink' : 'on'
  const choices = channels ? inputChoices(channels) : [shown]

  return (
    <div className='flex flex-row items-center gap-1'>
      <Led color='red' state={led} />
      <AnalogButton
        variant='text'
        size='xs'
        latched={armed !== null}
        aria-label={`Arm ${label} to record`}
        title={channels === null ? 'Arm to record (there\'s no input to record from)' : 'Arm to record'}
        onPress={() => onChange(armed ? null : shown)}
      >
        R
      </AnalogButton>
      <select
        className='ft-readout text-[10px] py-0 w-full min-w-0'
        aria-label={`${label} records from`}
        title='The input channels it records'
        value={inputKey(shown)}
        onChange={e => {
          const picked = parseInputKey(e.currentTarget.value)
          if(!picked) return
          setChoice(picked)
          if(armed) onChange(picked)
        }}
      >
        {choices.map(c => <option key={inputKey(c)} value={inputKey(c)}>{inputLabel(c)}</option>)}
      </select>
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
  // arming to record (track strips)
  record?: RecordControl
  // silenced by another track's solo
  silenced?: boolean
  // renaming (track strips). an empty name goes back to the default
  onRename?: (name: string) => void
  // the track's color, and picking another (track strips). ledRef is its
  // LED, for useMixerMeters to light while the track sounds
  color?: { value: number, onChange: (color: number) => void, ledRef?: (led: HTMLElement | null) => void }
  // the channel's effects: how many, whether any is on, and opening the rack
  fx?: { count: number, lit: boolean, onOpen: () => void }
  // the channel as dropped effects see it (see useChannelDrop)
  dropKey?: string
  // an effect dragged over the strip would go on it
  dropping?: boolean
  // under the fader, in place of mute and solo (the master strip)
  footer?: React.ReactNode
  // above the fader, in place of the pan control (the master strip)
  header?: React.ReactNode
  wide?: boolean
}

// one column of the mixer: name, pan, fader and meter, volume, mute and
// solo, and the effects' jack at the foot. a track's strip has its color as
// an LED jewel left of the name, which mutes and solos it too
const ChannelStrip = ({
  label,
  volumeDb,
  onVolume,
  meterKey,
  register,
  pan,
  mute,
  solo,
  record,
  silenced = false,
  onRename,
  color,
  fx,
  dropKey,
  dropping = false,
  footer,
  header,
  wide = false,
}: Props) => (
  <div
    className={`ft-strip flex flex-col items-stretch gap-2 h-full p-2 ${wide ? 'w-32' : 'w-28'} ${
      dropping ? 'ring-2 ring-inset ring-accent-focus bg-accent-tint' : ''}`}
    aria-label={label}
    role='group'
    {...(dropKey === undefined ? {} : { [DROP_CHANNEL]: dropKey })}
  >
    <div className='h-5 flex flex-row items-center gap-1 min-w-0'>
      {color && mute && solo && (
        <ChannelJewel
          label={label}
          color={color.value}
          onChange={color.onChange}
          muted={mute.on}
          soloed={solo.on}
          onMute={mute.onChange}
          onSolo={solo.onChange}
          ledRef={color.ledRef}
        />
      )}
      <div className='flex-1 min-w-0 h-full'>
        <NameLabel label={label} muted={!!mute?.on} silenced={silenced} onRename={onRename} />
      </div>
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

    {record && <RecordArm label={label} record={record} />}

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

    {fx && <FxJack channel={label} count={fx.count} lit={fx.lit} onOpen={fx.onOpen} />}
  </div>
)

export default ChannelStrip
