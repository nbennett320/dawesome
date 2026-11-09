import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  fetchMixer,
  resetMixer,
  selectAnySolo,
  selectMasterMix,
  selectTrackMix,
  selectTrackName,
  setMasterVolume,
  setTrackMute,
  setTrackName,
  setTrackPan,
  setTrackSolo,
  setTrackVolume,
} from '../../state/slices/mixerSlice'
import { selectPlaylistPlaying, selectPlaylistUI } from '../../state/slices/playlistSlice'
import ChannelStrip from './ChannelStrip'
import { MASTER_METER_KEY, RegisterMeter, trackMeterKey, useMixerMeters } from './useMixerMeters'

const TrackStrip = ({ trackNumber, register }: { trackNumber: number, register: RegisterMeter }) => {
  const dispatch = useAppDispatch()
  const mix = useAppSelector(selectTrackMix(trackNumber))
  const name = useAppSelector(selectTrackName(trackNumber))
  const anySolo = useAppSelector(selectAnySolo)

  return (
    <ChannelStrip
      label={name}
      onRename={newName => dispatch(setTrackName(trackNumber, newName))}
      volumeDb={mix.volumeDb}
      onVolume={db => dispatch(setTrackVolume(trackNumber, db))}
      meterKey={trackMeterKey(trackNumber)}
      register={register}
      pan={{ value: mix.pan, onChange: pan => dispatch(setTrackPan(trackNumber, pan)) }}
      mute={{ on: mix.mute, onChange: on => dispatch(setTrackMute(trackNumber, on)) }}
      solo={{ on: mix.solo, onChange: on => dispatch(setTrackSolo(trackNumber, on)) }}
      silenced={anySolo && !mix.solo && !mix.mute}
    />
  )
}

// resets every strip, after asking with a second click
const ResetAll = () => {
  const dispatch = useAppDispatch()
  const [asking, setAsking] = React.useState(false)

  // stop asking after a while, so a stray click later doesn't reset
  React.useEffect(() => {
    if(!asking) return undefined
    const timer = setTimeout(() => setAsking(false), 3000)
    return () => clearTimeout(timer)
  }, [asking])

  return (
    <button
      type='button'
      className={`flex-1 text-xs rounded border py-0.5 ${
        asking ? 'bg-red-600 border-red-700 text-white' : 'border-gray-300 text-gray-600 hover:bg-slate-100'
      }`}
      title='Put every fader, pan, mute and solo back to its default'
      onClick={() => {
        if(asking) {
          setAsking(false)
          dispatch(resetMixer()).catch(e => {
            // eslint-disable-next-line no-console
            console.error('could not reset the mixer:', e)
          })
        } else {
          setAsking(true)
        }
      }}
    >
      {asking ? 'Reset all?' : 'Reset all'}
    </button>
  )
}

// every playlist track's strip, summed into the master
const Mixer = () => {
  const dispatch = useAppDispatch()
  const { trackCount } = useAppSelector(selectPlaylistUI)
  const playing = useAppSelector(selectPlaylistPlaying)
  const master = useAppSelector(selectMasterMix)
  const { register, clipped, clearClip } = useMixerMeters(playing)

  // the engine owns the mixer. catch up with it when the tab opens and when
  // the playlist gains tracks
  React.useEffect(() => {
    dispatch(fetchMixer()).catch(e => {
      // eslint-disable-next-line no-console
      console.error('could not read the mixer:', e)
    })
  }, [dispatch, trackCount])

  const tracks = Array.from({ length: trackCount }, (_, i) => i)

  return (
    <div className='h-full w-full flex flex-row bg-slate-50 text-sm select-none overflow-y-auto'>
      <div className='flex-1 min-w-0 overflow-x-auto'>
        <div className='h-full flex flex-row gap-1 p-2 w-max' style={{ minHeight: 280 }}>
          {tracks.map(trackNumber => (
            <TrackStrip key={trackNumber} trackNumber={trackNumber} register={register} />
          ))}
        </div>
      </div>

      <div className='shrink-0 p-2 border-l border-gray-300' style={{ minHeight: 280 }}>
        <ChannelStrip
          label='Master'
          wide
          volumeDb={master.volumeDb}
          onVolume={db => dispatch(setMasterVolume(db))}
          meterKey={MASTER_METER_KEY}
          register={register}
          header={(
            <button
              type='button'
              className={`mt-auto text-[10px] font-semibold rounded border py-0.5 ${
                clipped ? 'bg-red-600 border-red-700 text-white' : 'border-gray-300 text-gray-400'
              }`}
              title={clipped
                ? 'The mix went over 0 dB, and the limiter rounded off its peaks. Click to clear'
                : 'Lights up if the mix goes over 0 dB'}
              onClick={clearClip}
            >
              CLIP
            </button>
          )}
          footer={<ResetAll />}
        />
      </div>
    </div>
  )
}

export default Mixer
