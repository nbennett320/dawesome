import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppDispatch } from '../../hooks/redux'
import { syncRun } from '../../state/slices/sequencerSlice'
import { setRecording } from '../../state/slices/recordingSlice'
import {
  AudioSettings as Settings,
  AudioStatus,
  bufferMs,
  bufferNote,
  meterPosition,
  msLabel,
  rateLabel,
} from '../../types/audio'
import { AnalogButton } from '../../components/fourtrack'
import { Bar, Choice, Field, Notice, ReadoutInput } from '../../components/common'
import { ResetIcon } from '../../components/icons'
import { clamp } from '../../util/helpers'

// how often the input meter is read, and how fast it falls
const METER_EVERY_MS = 50
const METER_FALL = 0.8

// the input's level, a bar a channel, while the tab is showing. the input is
// only open while this is (and while recording)
const InputMeter = ({ onOpened, onError }: { onOpened: () => void, onError: (e: string) => void }) => {
  const [levels, setLevels] = React.useState<number[]>([])

  React.useEffect(() => {
    let listening = false
    let stopped = false
    invoke<void>('listen_to_input', { on: true })
      .then(() => {
        listening = true
        if(stopped) invoke('listen_to_input', { on: false }).catch(() => {})
        else onOpened()
      })
      .catch(e => onError(`The input couldn't be opened: ${e}`))

    const timer = setInterval(() => {
      invoke<number[]>('get_input_levels')
        .then(peaks => setLevels(was => peaks.map((peak, i) => Math.max(peak, (was[i] ?? 0) * METER_FALL))))
        .catch(() => {})
    }, METER_EVERY_MS)

    return () => {
      stopped = true
      clearInterval(timer)
      if(listening) invoke('listen_to_input', { on: false }).catch(() => {})
    }
  }, [onOpened, onError])

  if(levels.length === 0) return null

  return (
    <div className='flex flex-col gap-1 w-72 max-w-full' role='group' aria-label='Input level'>
      {levels.map((level, i) => (
        // eslint-disable-next-line react/no-array-index-key
        <div key={i} className='flex flex-row items-center gap-2'>
          <span className='ft-label w-4 text-right'>{i + 1}</span>
          <Bar
            value={meterPosition(level)}
            tone={level >= 1 ? 'clip' : 'ok'}
            className='flex-1'
          />
        </div>
      ))}
    </div>
  )
}

// how much earlier recordings are placed, typed in milliseconds
const OffsetField = ({ value, disabled, onChange }: {
  value: number
  disabled: boolean
  onChange: (ms: number) => void
}) => (
  <div className='flex flex-col gap-1'>
    <Field label='Recording offset (ms)' htmlFor='audio-record-offset'>
      <ReadoutInput
        id='audio-record-offset'
        className='text-sm py-1 w-24'
        disabled={disabled}
        value={String(value)}
        onCommit={typed => {
          const ms = parseFloat(typed)
          if(Number.isFinite(ms) && ms !== value) onChange(clamp(ms, -1000, 1000))
        }}
      />
    </Field>
    <span className='text-xs text-ink-soft max-w-md'>
      If recordings land late, raise this by how much. Bluetooth headphones often need 150 to 250.
    </span>
  </div>
)

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
  const inputError = React.useCallback((e: string) => setError(e), [])

  const apply = (change: Partial<Settings>) => {
    if(!status) return
    setApplying(true)
    setError(null)
    invoke<AudioStatus>('set_audio_settings', { settings: { ...status.settings, ...change } })
      .then(applied => {
        setStatus(applied)
        // a change of devices or buffer stops any take being recorded
        dispatch(setRecording(false))
      })
      .catch(e => setError(String(e)))
      .finally(() => {
        setApplying(false)
        // and pauses playback
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
          <Notice tone='led'>{error ?? status.notice}</Notice>
        )}

        <section className='flex flex-col gap-3'>
          <Choice
            id='audio-output'
            label='Output'
            value={settings.outputDevice ?? ''}
            options={devices(status.outputs, settings.outputDevice)}
            disabled={disabled}
            onChange={name => apply({ outputDevice: name || null })}
          />
          <Choice
            id='audio-buffer-size'
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
          <p className='text-xs text-ink-soft max-w-md'>
            Smaller buffers mean less delay, but are more work for the computer. 
            If playback crackles, choose a bigger one.
          </p>
        </section>

        <section className='flex flex-col gap-3'>
          <Choice
            id='audio-input'
            label='Input'
            value={settings.inputDevice ?? ''}
            options={devices(status.inputs, settings.inputDevice)}
            disabled={disabled}
            onChange={name => apply({ inputDevice: name || null })}
          />
          <InputMeter onOpened={load} onError={inputError} />
          {status.input && (
            <span className='text-xs text-ink-soft'>
              Input latency {msLabel(status.input.latencyMs)} · {rateLabel(status.input.sampleRate)}
              {output && status.input.sampleRate !== output.sampleRate
                && ` (it can't record at ${rateLabel(output.sampleRate)}, so takes are converted as they play)`}
            </span>
          )}
          <OffsetField
            value={settings.recordOffsetMs}
            disabled={disabled}
            onChange={ms => apply({ recordOffsetMs: ms })}
          />
        </section>

        <section className='flex flex-col gap-1'>
          <span className='ft-label'>Playback Status</span>
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

          {note && <span className='text-xs text-ink-soft'>{note}</span>}
        </section>

        <div className='flex flex-row gap-2 items-center'>
          {/* <button
            type='button'
            className='rounded px-2 py-1 hover:bg-panel text-ink-mid'
            disabled={disabled}
            onClick={load}
          >
            Refresh devices
          </button> */}
          <span className='text-xs'>Refresh devices</span>  
          <AnalogButton size='md' aria-label='Refresh samples' title='List the samples folder again' onPress={load} disabled={disabled}>
            <ResetIcon />
          </AnalogButton>

          {applying && <span className='text-xs text-ink-soft'>Switching…</span>}
        </div>
      </div>
    </div>
  )
}

export default AudioSettings
