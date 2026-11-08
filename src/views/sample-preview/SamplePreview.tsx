import React from 'react'
import { invoke } from '@tauri-apps/api'
import {
  auditionClip,
  resetClipSettings,
  selectClipSampleDuration,
  selectClipSettings,
  selectInspectedClip,
  selectPlaylistTempo,
  updateClipSettings,
} from '../../state/slices/playlistSlice'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  ClipSetting,
  ClipSettings,
  clipSettingRanges,
  defaultClipSettings,
  isDefaultClipSettings,
  playbackRate,
  playedDuration,
} from '../../types/clip'
import ClipWaveform, { WaveformData } from './ClipWaveform'
import SettingControl from './SettingControl'

type Loaded =
  | { state: 'loading' }
  | { state: 'error', message: string }
  | { state: 'ready', points: number[] }

// the inspected clip's waveform points, from the engine
const useClipWaveform = (id: number | undefined): Loaded | null => {
  const [loaded, setLoaded] = React.useState<Loaded | null>(null)

  React.useEffect(() => {
    if(id === undefined) {
      setLoaded(null)
      return undefined
    }

    // ignore the answer if another clip was inspected before it arrived
    let current = true
    setLoaded({ state: 'loading' })

    // (the duration it also returns is the played length, which changes with
    // the clip's pitch. the sample's own length comes from get_clip)
    invoke<[number[], number]>('get_node_data', { id })
      .then(([points]) => {
        if(current) setLoaded({ state: 'ready', points })
      })
      .catch(e => {
        if(current) setLoaded({ state: 'error', message: String(e) })
      })

    return () => { current = false }
  }, [id])

  return loaded
}

// round for display, dropping trailing zeros
const round = (n: number, places = 2) => Number(n.toFixed(places))

const panHint = (pan: number) => {
  if(Math.abs(pan) < 0.005) return 'Centre'
  return `${Math.round(Math.abs(pan) * 100)}% ${pan < 0 ? 'left' : 'right'}`
}

const signed = (n: number) => (n > 0 ? `+${round(n)}` : String(round(n)))

// inspect a clip from the playlist and adjust how it plays
const SamplePreview = () => {
  const dispatch = useAppDispatch()
  const clip = useAppSelector(selectInspectedClip)
  const tempo = useAppSelector(selectPlaylistTempo)
  const settings = useAppSelector(selectClipSettings(clip?.id))
  const sampleDuration = useAppSelector(selectClipSampleDuration(clip?.id))
  const waveform = useClipWaveform(clip?.id)

  if(!clip) {
    return (
      <div className='h-full w-full flex items-center justify-center text-sm text-gray-500'>
        Double-click a clip in the playlist to inspect it.
      </div>
    )
  }

  const name = clip.path.split(/[\\/]/).pop() ?? clip.path
  const played = sampleDuration === undefined ? null : playedDuration(settings, sampleDuration)
  const waveformData: WaveformData | null = waveform?.state === 'ready' && sampleDuration !== undefined
    ? { points: waveform.points, duration: sampleDuration }
    : null

  const update = (change: Partial<ClipSettings>) => {
    dispatch(updateClipSettings(clip.id, change))
  }

  // the props every control shares, for one setting
  const control = (key: ClipSetting) => ({
    value: settings[key],
    min: clipSettingRanges[key].min,
    max: clipSettingRanges[key].max,
    onChange: (value: number) => update({ [key]: value }),
    onReset: () => update({ [key]: defaultClipSettings[key] }),
  })

  // fades can't be longer than the clip
  const fadeMax = played === null ? clipSettingRanges.attackMs.max : Math.round(played * 1000)

  return (
    <div className='h-full w-full p-4 flex flex-col gap-3 text-sm overflow-auto'>
      <div className='flex flex-row items-start justify-between gap-4'>
        <div className='min-w-0'>
          <h2 className='text-base font-medium truncate' title={clip.path}>{name}</h2>

          <div className='mt-1 flex flex-row flex-wrap gap-x-4 gap-y-1 text-xs text-gray-500'>
            {/* tracks are numbered from 0 in the playlist */}
            <span>Track <span className='text-gray-800'>{clip.trackNumber + 1}</span></span>
            <span>Starts at beat <span className='text-gray-800'>{round(clip.startBeat)}</span></span>
            {played !== null && (
              <span>
                Length <span className='text-gray-800'>{round(played)} s</span>
                {' '}({round(played * tempo / 60)} beats)
              </span>
            )}
          </div>
        </div>

        <div className='flex flex-row gap-2 shrink-0'>
          <button
            className='px-2 py-1 text-xs rounded border border-gray-300 hover:bg-slate-100'
            title='Play the clip with its settings'
            onClick={() => { dispatch(auditionClip(clip.id)) }}
          >
            Audition
          </button>
          <button
            className='px-2 py-1 text-xs rounded border border-gray-300 hover:bg-slate-100 disabled:opacity-40'
            disabled={isDefaultClipSettings(settings)}
            onClick={() => dispatch(resetClipSettings(clip.id))}
          >
            Reset
          </button>
        </div>
      </div>

      <div
        className='flex-1 rounded border border-gray-300 overflow-hidden'
        style={{ minHeight: 140 }}
      >
        {waveformData && (
          <ClipWaveform
            data={waveformData}
            settings={settings}
            tempo={tempo}
            onEnvelopeChange={update}
          />
        )}
        {!waveformData && waveform?.state !== 'error' && (
          <div className='h-full flex items-center justify-center text-xs text-gray-500'>
            Loading waveform…
          </div>
        )}
        {waveform?.state === 'error' && (
          <div className='h-full flex items-center justify-center text-xs text-red-700'>
            Couldn&apos;t load the waveform: {waveform.message}
          </div>
        )}
      </div>

      <div
        className='grid gap-x-6 gap-y-3'
        style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(14rem, 1fr))' }}
      >
        <SettingControl
          label='Gain'
          {...control('gainDb')}
          step={0.1}
          unit='dB'
        />
        <SettingControl
          label='Pan'
          {...control('pan')}
          step={0.01}
          fieldScale={100}
          unit='%'
          hint={panHint(settings.pan)}
        />
        <SettingControl
          label='Pitch / speed'
          {...control('pitch')}
          step={0.1}
          unit='st'
          hint={`${signed(settings.pitch)} st, ×${round(playbackRate(settings), 3)} speed`}
        />
        <SettingControl
          label='Attack'
          {...control('attackMs')}
          max={fadeMax}
          step={1}
          unit='ms'
          hint='Fade in'
        />
        <SettingControl
          label='Decay'
          {...control('decayMs')}
          max={fadeMax}
          step={1}
          unit='ms'
          hint='Fade out'
        />
      </div>
    </div>
  )
}

export default SamplePreview
