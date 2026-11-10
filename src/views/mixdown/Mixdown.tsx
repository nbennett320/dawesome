import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectProjectPath } from '../../state/slices/projectSlice'
import {
  Bits, BITS_LABELS, chosenPlace, FileType, formatOf, Kbps, KBPS,
  selectMixdown, setBits, setFileType, setKbps, setMixdownPlace, setWatching,
} from '../../state/slices/mixdownSlice'
import { AnalogButton } from '../../components/fourtrack'

// how a mixdown is going (see Status in engine/src/mixdown_commands.rs)
type Status =
  | { state: 'idle' }
  | { state: 'rendering', progress: number }
  | { state: 'done', path: string }
  | { state: 'failed', message: string }
  | { state: 'cancelled' }

// how often the engine is asked, while a mixdown is going
const POLL_MS = 100

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path

const Field = ({ label, htmlFor, children }: { label: string, htmlFor: string, children: React.ReactNode }) => (
  <div className='flex flex-col gap-1'>
    <label htmlFor={htmlFor} className='ft-label'>{label}</label>
    {children}
  </div>
)

const Button = ({ children, ...props }: React.ComponentProps<typeof AnalogButton>) => (
  <AnalogButton variant='text' size='sm' {...props}>{children}</AnalogButton>
)

// File › Mixdown…: the whole song, as it's heard, to a WAV or MP3 file
const Mixdown = () => {
  const dispatch = useAppDispatch()
  const mixdown = useAppSelector(selectMixdown)
  const project = useAppSelector(selectProjectPath)
  const [defaults, setDefaults] = React.useState<{ name: string, dir: string } | null>(null)
  const [status, setStatus] = React.useState<Status>({ state: 'idle' })
  const [error, setError] = React.useState<string | null>(null)
  const [starting, setStarting] = React.useState(false)

  // the project's name and folder, again whenever another is opened or it's saved
  React.useEffect(() => {
    invoke<{ name: string, dir: string }>('mixdown_defaults')
      .then(setDefaults)
      .catch(e => setError(String(e)))
  }, [project])

  const refresh = React.useCallback(() => {
    invoke<Status>('mixdown_status').then(setStatus).catch(() => {})
  }, [])
  // (one may be going from before the tab was opened)
  React.useEffect(refresh, [refresh])
  const rendering = status.state === 'rendering'
  React.useEffect(() => {
    if(!rendering) return undefined
    const timer = setInterval(refresh, POLL_MS)
    return () => clearInterval(timer)
  }, [rendering, refresh])

  const chosen = chosenPlace(mixdown, project)
  const name = chosen.name ?? defaults?.name ?? ''
  const dir = chosen.dir ?? defaults?.dir ?? ''

  const start = () => {
    setError(null)
    setStarting(true)
    invoke<void>('start_mixdown', { options: { name, dir, format: formatOf(mixdown) } })
      .then(() => {
        dispatch(setWatching(true))
        setStatus({ state: 'rendering', progress: 0 })
      })
      .catch(e => setError(String(e)))
      .finally(() => setStarting(false))
  }

  const browse = () => {
    open({ directory: true, defaultPath: dir || undefined, title: 'Mix down to' })
      .then(chosenDir => {
        if(typeof chosenDir === 'string') dispatch(setMixdownPlace({ project, dir: chosenDir }))
      })
      .catch(e => setError(String(e)))
  }

  if(rendering) {
    const percent = Math.round(status.progress * 100)
    return (
      <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
        <div className='flex flex-col gap-3 max-w-xl'>
          <span className='ft-label'>Mixing down…</span>
          <div
            className='h-2 w-72 max-w-full rounded-sm bg-slate-200 overflow-hidden'
            role='progressbar'
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
          >
            <div className='h-full bg-lime-500' style={{ width: `${percent}%` }} />
          </div>
          <span className='ft-readout text-sm w-fit'>{percent}%</span>
          <div>
            <Button title='Stop, and keep nothing of it' onPress={() => { invoke('cancel_mixdown').catch(() => {}) }}>
              Cancel
            </Button>
          </div>
        </div>
      </div>
    )
  }

  if(mixdown.watching && status.state === 'done') {
    return (
      <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
        <div className='flex flex-col gap-3 max-w-xl'>
          <span className='ft-label'>Done</span>
          <span className='ft-lcd text-xs w-fit'>{fileName(status.path)}</span>
          <span className='text-xs text-slate-500 break-all select-text'>{status.path}</span>
          {error && (
            <div role='alert' className='rounded border border-amber-300 bg-amber-50 px-3 py-2 text-amber-900'>
              {error}
            </div>
          )}
          <div className='flex flex-row gap-2'>
            <Button onPress={() => { invoke('reveal_file', { path: status.path }).catch(e => setError(String(e))) }}>
              Show in Finder
            </Button>
            <Button onPress={() => { setError(null); dispatch(setWatching(false)) }}>
              New mixdown
            </Button>
          </div>
        </div>
      </div>
    )
  }

  // what the last mixdown came to, when it didn't finish
  const outcomes: Partial<Record<Status['state'], string>> = {
    failed: status.state === 'failed' ? `The mixdown failed: ${status.message}` : undefined,
    cancelled: 'The mixdown was cancelled.',
  }
  const outcome = mixdown.watching ? outcomes[status.state] ?? null : null
  const disabled = starting || !defaults
  const extension = mixdown.fileType

  return (
    <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
      <form
        className='flex flex-col gap-4 max-w-xl'
        onSubmit={e => {
          e.preventDefault()
          if(!disabled && name.trim()) start()
        }}
      >
        {(error || outcome) && (
          <div role='alert' className='rounded border border-amber-300 bg-amber-50 px-3 py-2 text-amber-900'>
            {error ?? outcome}
          </div>
        )}

        <Field label='Name' htmlFor='mixdown-name'>
          <div className='flex flex-row items-center gap-1'>
            <input
              id='mixdown-name'
              type='text'
              className='ft-readout text-sm py-1 text-left w-72 max-w-full'
              value={name}
              disabled={disabled}
              onChange={e => dispatch(setMixdownPlace({ project, name: e.currentTarget.value }))}
            />
            <span className='text-xs text-slate-500'>.{extension}</span>
          </div>
        </Field>

        <Field label='Path' htmlFor='mixdown-dir'>
          <div className='flex flex-row items-center gap-2'>
            <input
              id='mixdown-dir'
              type='text'
              readOnly
              className='ft-readout text-sm py-1 text-left w-72 max-w-full'
              value={dir}
              title={dir}
              disabled={disabled}
            />
            <Button disabled={disabled} onPress={browse}>Browse…</Button>
          </div>
        </Field>

        <div className='flex gap-4'>
          <div>
            <Field label='File type' htmlFor='mixdown-type'>
              <select
                id='mixdown-type'
                className='ft-readout text-sm py-1 text-left w-72 max-w-full'
                value={mixdown.fileType}
                disabled={disabled}
                onChange={e => dispatch(setFileType(e.currentTarget.value as FileType))}
              >
                <option value='wav'>WAV</option>
                <option value='mp3'>MP3</option>
              </select>
            </Field>
          </div>

          <div>
            {mixdown.fileType === 'wav' ? (
              <Field label='Bit depth' htmlFor='mixdown-bits'>
                <select
                  id='mixdown-bits'
                  className='ft-readout text-sm py-1 text-left w-72 max-w-full'
                  value={mixdown.bits}
                  disabled={disabled}
                  onChange={e => dispatch(setBits(e.currentTarget.value as Bits))}
                >
                  {(Object.keys(BITS_LABELS) as Bits[]).map(bits => (
                    <option key={bits} value={bits}>{BITS_LABELS[bits]}</option>
                  ))}
                </select>
              </Field>
            ) : (
              <Field label='Bitrate' htmlFor='mixdown-kbps'>
                <select
                  id='mixdown-kbps'
                  className='ft-readout text-sm py-1 text-left w-72 max-w-full'
                  value={mixdown.kbps}
                  disabled={disabled}
                  onChange={e => dispatch(setKbps(Number(e.currentTarget.value) as Kbps))}
                >
                  {KBPS.map(kbps => <option key={kbps} value={kbps}>{kbps} kbps</option>)}
                </select>
              </Field>
            )}
          </div>
        </div>

        <div>
          <Button type='submit' disabled={disabled || !name.trim()}>
            {starting ? 'Starting…' : 'Confirm'}
          </Button>
        </div>
      </form>
    </div>
  )
}

export default Mixdown
