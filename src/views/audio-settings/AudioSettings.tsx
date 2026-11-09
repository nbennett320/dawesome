import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useAppDispatch } from '../../hooks/redux'
import { syncRun } from '../../state/slices/sequencerSlice'
import {
  AudioSettings as Settings,
  AudioStatus,
  bufferMs,
  bufferNote,
  msLabel,
  rateLabel,
} from '../../types/audio'

// a choice of devices (or the system's default), or buffer sizes
const Choice = ({ label, value, options, disabled, onChange }: {
  label: string
  // '' for the default
  value: string
  options: Array<{ value: string, label: string }>
  disabled: boolean
  onChange: (value: string) => void
}) => {
  const id = `audio-${label.toLowerCase().replace(/\s+/g, '-')}`
  return (
    <div className='flex flex-col gap-1'>
      <label htmlFor={id} className='ft-label'>{label}</label>
      <select
        id={id}
        className='ft-readout text-sm py-1 text-left w-72 max-w-full'
        value={value}
        disabled={disabled}
        onChange={e => onChange(e.currentTarget.value)}
      >
        {options.map(option => (
          <option key={option.value} value={option.value}>{option.label}</option>
        ))}
      </select>
    </div>
  )
}

const devices = (names: string[], chosen: string | null) => [
  { value: '', label: 'System default' },
  // a saved device that isn't plugged in stays listed, so it shows as chosen
  ...[...names, ...(chosen && !names.includes(chosen) ? [chosen] : [])]
    .map(name => ({ value: name, label: names.includes(name) ? name : `${name} (not connected)` })),
]

// the devices the engine plays (and records) through, and its buffer size
const AudioSettings = () => {
  const dispatch = useAppDispatch()
  const [status, setStatus] = React.useState<AudioStatus | null>(null)
  const [applying, setApplying] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)

  const load = React.useCallback(() => {
    invoke<AudioStatus>('get_audio_settings')
      .then(setStatus)
      .catch(e => setError(`The audio settings couldn't be read: ${e}`))
  }, [])

  React.useEffect(load, [load])

  const apply = (change: Partial<Settings>) => {
    if(!status) return
    setApplying(true)
    setError(null)
    invoke<AudioStatus>('set_audio_settings', { settings: { ...status.settings, ...change } })
      .then(setStatus)
      .catch(e => setError(String(e)))
      .finally(() => {
        setApplying(false)
        // a change of output pauses playback
        syncRun(dispatch).catch(() => {})
      })
  }

  if(!status) {
    return (
      <div className='ft-panel h-full w-full p-4 text-sm'>
        {error ?? 'Looking for audio devices…'}
      </div>
    )
  }

  const { settings, output } = status
  const disabled = applying
  const note = bufferNote(settings, output)

  return (
    <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
      <div className='flex flex-col gap-4 max-w-xl'>
        {(error || status.notice) && (
          <div role='alert' className='rounded border border-amber-300 bg-amber-50 px-3 py-2 text-amber-900'>
            {error ?? status.notice}
          </div>
        )}

        <section className='flex flex-col gap-3'>
          <Choice
            label='Output'
            value={settings.outputDevice ?? ''}
            options={devices(status.outputs, settings.outputDevice)}
            disabled={disabled}
            onChange={name => apply({ outputDevice: name || null })}
          />
          <Choice
            label='Buffer size'
            value={settings.bufferFrames?.toString() ?? ''}
            options={[
              { value: '', label: 'Device default' },
              ...status.bufferChoices.map(frames => ({
                value: frames.toString(),
                label: output
                  ? `${frames} frames (${msLabel(bufferMs(frames, output.sampleRate))})`
                  : `${frames} frames`,
              })),
            ]}
            disabled={disabled}
            onChange={frames => apply({ bufferFrames: frames ? Number(frames) : null })}
          />
          <p className='text-xs text-slate-500 max-w-md'>
            Smaller buffers mean less delay, but are more work for the computer. If playback
            crackles, choose a bigger one.
          </p>
        </section>

        <section className='flex flex-col gap-3'>
          <Choice
            label='Input'
            value={settings.inputDevice ?? ''}
            options={devices(status.inputs, settings.inputDevice)}
            disabled={disabled}
            onChange={name => apply({ inputDevice: name || null })}
          />
          <p className='text-xs text-slate-500 max-w-md'>
            What recording listens to.
          </p>
        </section>

        <section className='flex flex-col gap-1'>
          <span className='ft-label'>Now playing through</span>
          {output ? (
            <div className='ft-lcd text-xs flex flex-col gap-0.5 w-fit'>
              <span>{output.device}</span>
              <span>
                {rateLabel(output.sampleRate)}
                {' · '}
                {output.bufferFrames > 0
                  ? `${output.bufferFrames} frame buffer (${msLabel(bufferMs(output.bufferFrames, output.sampleRate))})`
                  : 'starting…'}
              </span>
              <span>Output latency {msLabel(output.latencyMs)}</span>
            </div>
          ) : (
            <span>Nothing: the output couldn&apos;t be opened</span>
          )}
          {note && <span className='text-xs text-slate-500'>{note}</span>}
          <span className='text-xs text-slate-500 max-w-md'>
            {"Bluetooth headphones add delay that they don't report, so recordings made while "
              + 'listening through them can land late.'}
          </span>
        </section>

        <div className='flex flex-row gap-2 items-center'>
          <button
            type='button'
            className='rounded px-2 py-1 hover:bg-slate-100 text-slate-600'
            disabled={disabled}
            onClick={load}
          >
            Refresh devices
          </button>
          {applying && <span className='text-xs text-slate-500'>Switching…</span>}
        </div>
      </div>
    </div>
  )
}

export default AudioSettings
