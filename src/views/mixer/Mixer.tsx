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
import {
  fetchRecordSetup,
  selectArmed,
  selectInputChannels,
  selectRecording,
  setTrackArmed,
} from '../../state/slices/recordingSlice'
import ChannelStrip from './ChannelStrip'
import { AnalogButton, Led } from '../../components/fourtrack'
import { MASTER_METER_KEY, RegisterMeter, trackMeterKey, useMixerMeters } from './useMixerMeters'

const TrackStrip = ({ trackNumber, register }: { trackNumber: number, register: RegisterMeter }) => {
  const dispatch = useAppDispatch()
  const mix = useAppSelector(selectTrackMix(trackNumber))
  const name = useAppSelector(selectTrackName(trackNumber))
  const anySolo = useAppSelector(selectAnySolo)
  const armed = useAppSelector(selectArmed(trackNumber))
  const channels = useAppSelector(selectInputChannels)
  const recording = useAppSelector(selectRecording)

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
      record={{
        armed,
        channels,
        recording,
        onChange: input => {
          dispatch(setTrackArmed(trackNumber, input)).catch(e => {
            // eslint-disable-next-line no-console
            console.error('could not arm the track:', e)
          })
        },
      }}
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
    <AnalogButton
      variant='text'
      size='sm'
      className={`self-end ${asking ? 'ft-btn--danger' : ''}`}
      title='Put every fader, pan, mute and solo back to its default'
      onPress={() => {
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
      {asking ? 'Sure?' : 'Reset'}
    </AnalogButton>
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
    // and which tracks are armed, and the input's channels
    dispatch(fetchRecordSetup()).catch(e => {
      // eslint-disable-next-line no-console
      console.error('could not read the recording setup:', e)
    })
  }, [dispatch, trackCount])

  const tracks = Array.from({ length: trackCount }, (_, i) => i)

  return (
    <div className='ft-panel h-full w-full flex flex-row text-sm select-none overflow-y-auto'>
      <div className='flex-1 min-w-0 overflow-x-auto'>
        <div className='ft-tray rounded-lg m-2 h-[calc(100%-1rem)] flex flex-row gap-0.5 p-0.5 w-max' style={{ minHeight: 320 }}>
          {tracks.map(trackNumber => (
            <TrackStrip key={trackNumber} trackNumber={trackNumber} register={register} />
          ))}
        </div>
      </div>

      <div className='shrink-0 py-2 pr-2' style={{ minHeight: 336 }}>
        <div className='ft-tray rounded-lg h-full p-0.5'>
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
              className='mt-auto flex flex-row items-center gap-1.5 rounded px-1 py-0.5 hover:bg-panel'
              title={clipped
                ? 'The mix went over 0 dB, and the limiter rounded off its peaks. Click to clear'
                : 'Lights up if the mix goes over 0 dB'}
              onClick={clearClip}
            >
              <Led color='red' state={clipped ? 'on' : 'off'} />
              <span className={`ft-label ${clipped ? 'text-danger' : ''}`}>Clip</span>
            </button>
          )}
          footer={<ResetAll />}
        />
        </div>
      </div>
    </div>
  )
}

export default Mixer
