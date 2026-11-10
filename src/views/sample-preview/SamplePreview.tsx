import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  auditionClip,
  resetClipSettings,
  selectClipSampleDuration,
  selectClipSettings,
  selectClipWindow,
  selectInspectedClip,
  selectPlaylistTempo,
  updateClipSettings,
} from '../../state/slices/playlistSlice'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectTrackName } from '../../state/slices/mixerSlice'
import {
  ClipSetting,
  ClipSettings,
  clipSettingRanges,
  defaultClipSettings,
  isDefaultClipSettings,
  playbackRate,
  windowedDuration,
} from '../../types/clip'
import ClipWaveform, { WaveformData } from './ClipWaveform'
import SettingControl from './SettingControl'
import { AnalogButton, PlayIcon } from '../../components/fourtrack'
import { decodeLevels } from '../../util/waveform'

type Loaded =
  | { state: 'loading' }
  | { state: 'error', message: string }
  | { state: 'ready', levels: Float32Array }

// the inspected clip's waveform levels, from the engine
// - path: the file it plays, which a deep copy or a join changes under the
//   same clip: its waveform is asked for again then
const useClipWaveform = (id: number | undefined, path: string | undefined): Loaded | null => {
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
    invoke<[string, number]>('get_node_data', { id })
      .then(([encoded]) => {
        if(current) setLoaded({ state: 'ready', levels: decodeLevels(encoded) })
      })
      .catch(e => {
        if(current) setLoaded({ state: 'error', message: String(e) })
      })

    return () => { current = false }
  }, [id, path])

  return loaded
}

// round for display, dropping trailing zeros
const round = (n: number, places = 2) => Number(n.toFixed(places))

const panHint = (pan: number) => {
  if(Math.abs(pan) < 0.005) return 'Centre'
  return `${Math.round(Math.abs(pan) * 100)}% ${pan < 0 ? 'left' : 'right'}`
}

const signed = (n: number) => (n > 0 ? `+${round(n)}` : String(round(n)))

// a fact about the clip: a small label over a readout
const Fact = ({ label, children }: { label: string, children: React.ReactNode }) => (
  <div className='flex flex-col gap-1 min-w-0'>
    <span className='ft-label text-[9px]'>{label}</span>
    <span className='ft-readout text-left px-2 truncate'>{children}</span>
  </div>
)

// inspect a clip from the playlist and adjust how it plays
const SamplePreview = () => {
  const dispatch = useAppDispatch()
  const clip = useAppSelector(selectInspectedClip)
  const tempo = useAppSelector(selectPlaylistTempo)
  const settings = useAppSelector(selectClipSettings(clip?.id))
  const clipWindow = useAppSelector(selectClipWindow(clip?.id))
  const sampleDuration = useAppSelector(selectClipSampleDuration(clip?.id))
  const waveform = useClipWaveform(clip?.id, clip?.path)
  const trackName = useAppSelector(selectTrackName(clip?.trackNumber ?? 0))

  if(!clip) {
    return (
      <div className='ft-panel h-full w-full flex items-center justify-center text-sm text-slate-400'>
        Double-click a clip in the playlist to inspect it.
      </div>
    )
  }

  const name = clip.path.split(/[\\/]/).pop() ?? clip.path
  const played = sampleDuration === undefined ? null : windowedDuration(settings, sampleDuration, clipWindow)
  const waveformData: WaveformData | null = waveform?.state === 'ready' && sampleDuration !== undefined
    ? { levels: waveform.levels, duration: sampleDuration }
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
    <div className='ft-panel h-full w-full p-4 flex flex-col gap-3 text-sm overflow-auto'>
      <div className='flex flex-row items-end justify-between gap-4'>
        <div className='min-w-0 flex flex-col gap-2'>
          <h2 className='text-base font-medium text-slate-700 truncate' title={clip.path}>{name}</h2>

          <div className='flex flex-row flex-wrap gap-3'>
            {played !== null && (
              <Fact label='Length'>{round(played)} s · {round(played * tempo / 60)} beats</Fact>
            )}
          </div>
        </div>

        <div className='ft-tray rounded-lg flex flex-row items-end gap-2 px-2.5 pt-2.5 pb-1.5 shrink-0'>
          <AnalogButton
            size='sm'
            aria-label='Audition'
            title='Play the clip with its settings'
            onPress={() => { dispatch(auditionClip(clip.id)) }}
          >
            <PlayIcon />
          </AnalogButton>
          <AnalogButton
            variant='text'
            size='sm'
            className='h-[34px]'
            title='Put every setting back to its default'
            disabled={isDefaultClipSettings(settings)}
            onPress={() => dispatch(resetClipSettings(clip.id))}
          >
            Reset
          </AnalogButton>
        </div>
      </div>

      <div
        className='ft-strip flex-1 overflow-hidden'
        style={{ minHeight: 140 }}
      >
        {waveformData && (
          <ClipWaveform
            data={waveformData}
            settings={settings}
            tempo={tempo}
            window={clipWindow}
            onEnvelopeChange={update}
          />
        )}
        {!waveformData && waveform?.state !== 'error' && (
          <div className='h-full flex items-center justify-center text-xs text-slate-400'>
            Loading waveform…
          </div>
        )}
        {waveform?.state === 'error' && (
          <div className='h-full flex items-center justify-center text-xs text-red-600'>
            Couldn&apos;t load the waveform: {waveform.message}
          </div>
        )}
      </div>

      <div className='ft-tray rounded-lg self-start flex flex-row flex-wrap gap-0.5 p-0.5'>
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
          label='Pitch'
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
          taper={3}
          hint='Fade in'
        />
        <SettingControl
          label='Decay'
          {...control('decayMs')}
          max={fadeMax}
          step={1}
          unit='ms'
          taper={3}
          hint='Fade out'
        />
      </div>
    </div>
  )
}

export default SamplePreview
