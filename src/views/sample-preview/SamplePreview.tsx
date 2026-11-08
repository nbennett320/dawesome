import React from 'react'
import { invoke } from '@tauri-apps/api'
import { selectInspectedClip, selectPlaylistTempo } from '../../state/slices/playlistSlice'
import { useAppSelector } from '../../hooks/redux'
import ClipWaveform, { WaveformData } from './ClipWaveform'

type Loaded =
  | { state: 'loading' }
  | { state: 'error', message: string }
  | { state: 'ready', data: WaveformData }

// the inspected clip's waveform and length, from the engine
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

    invoke<[number[], number]>('get_node_data', { id })
      .then(([points, duration]) => {
        if(current) setLoaded({ state: 'ready', data: { points, duration } })
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

// inspect a clip from the playlist and adjust how it plays
// (the clip's details and waveform for now, the settings come next)
const SamplePreview = () => {
  const clip = useAppSelector(selectInspectedClip)
  const tempo = useAppSelector(selectPlaylistTempo)
  const waveform = useClipWaveform(clip?.id)

  if(!clip) {
    return (
      <div className='h-full w-full flex items-center justify-center text-sm text-gray-500'>
        Double-click a clip in the playlist to inspect it.
      </div>
    )
  }

  const name = clip.path.split(/[\\/]/).pop() ?? clip.path
  const duration = waveform?.state === 'ready' ? waveform.data.duration : null

  return (
    <div className='h-full w-full p-4 flex flex-col gap-3 text-sm'>
      <div>
        <h2 className='text-base font-medium truncate' title={clip.path}>{name}</h2>

        <div className='mt-1 flex flex-row flex-wrap gap-x-4 gap-y-1 text-xs text-gray-500'>
          {/* tracks are numbered from 0 in the playlist */}
          <span>Track <span className='text-gray-800'>{clip.trackNumber + 1}</span></span>
          <span>Starts at beat <span className='text-gray-800'>{round(clip.startBeat)}</span></span>
          {duration !== null && (
            <span>
              Length <span className='text-gray-800'>{round(duration)} s</span>
              {' '}({round(duration * tempo / 60)} beats)
            </span>
          )}
        </div>
      </div>

      <div
        className='flex-1 rounded border border-gray-300 overflow-hidden'
        style={{ minHeight: 120 }}
      >
        {waveform?.state === 'ready' && <ClipWaveform data={waveform.data} tempo={tempo} />}
        {waveform?.state === 'loading' && (
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
    </div>
  )
}

export default SamplePreview
