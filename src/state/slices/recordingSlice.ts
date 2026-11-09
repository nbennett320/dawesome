import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import type { RootState } from '../store'
import { InputChoice, RecordSetup } from '../../types/audio'
import { LoadedProject, Recorded } from '../../types/project'
import { setProjectNotice } from './projectSlice'
import { playlistSlice, reduceLoadedProject, syncPlayingFrom } from './playlistSlice'

// recording onto the playlist (see engine/src/recording.rs): which tracks are
// armed and what each records, and whether a take is being recorded
export interface RecordingState {
  recording: boolean
  // input choices, by track number
  armed: Record<number, InputChoice>
  // the input's channels, or null when there's no input
  inputChannels: number | null
  inputError: string | null
}

const initialState: RecordingState = {
  recording: false,
  armed: {},
  inputChannels: null,
  inputError: null,
}

export const recordingSlice = createSlice({
  name: 'recordingSlice',
  initialState,
  reducers: {
    setRecording: (state, action: PayloadAction<boolean>) => {
      state.recording = action.payload
    },
    setRecordSetup: (state, action: PayloadAction<RecordSetup>) => {
      const { armed, inputChannels, inputError } = action.payload
      state.armed = Object.fromEntries(armed.map(track => [track.trackNumber, track.input]))
      state.inputChannels = inputChannels
      state.inputError = inputError
    },
  },
  extraReducers: builder => {
    // a project brings its own armed tracks
    builder.addCase(reduceLoadedProject, (state, action: PayloadAction<LoadedProject>) => {
      state.armed = Object.fromEntries((action.payload.armed ?? []).map(track => [track.trackNumber, track.input]))
    })
  },
})

export const { setRecording, setRecordSetup } = recordingSlice.actions

export const selectRecording = (state: RootState) => state.recording.recording
export const selectArmed = (trackNumber: number) => (state: RootState): InputChoice | null =>
  state.recording.armed[trackNumber] ?? null
export const selectAnyArmed = (state: RootState) => Object.keys(state.recording.armed).length > 0
export const selectInputChannels = (state: RootState) => state.recording.inputChannels

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// the armed tracks, and the input's channels to choose from
export const fetchRecordSetup = () => async (dispatch: Dispatch) => {
  dispatch(setRecordSetup(await invoke<RecordSetup>('get_record_setup')))
}

// arm a track to record these input channels, or (null) disarm it
export const setTrackArmed = (trackNumber: number, input: InputChoice | null) => async (dispatch: Dispatch) => {
  dispatch(setRecordSetup(await invoke<RecordSetup>('set_track_armed', { trackNumber, input })))
}

// put recorded takes on the playlist, and say what went wrong
const receive = (dispatch: Dispatch, recorded: Recorded) => {
  dispatch(setRecording(recorded.recording))
  recorded.clips.forEach(({ id, path, trackNumber, startBeat, settings, sampleDuration, playedDuration }) => {
    dispatch(playlistSlice.actions.addPlaylistItem({
      id,
      path,
      startBeat,
      trackNumber,
      pixelOffset: { x: 0, y: 0, left: 0, top: 0, xOffset: 0, yOffset: 0 },
    }))
    dispatch(playlistSlice.actions.setClipInfo({ id, settings, sampleDuration, playedDuration }))
  })
  if(recorded.problems.length > 0) {
    dispatch(setProjectNotice({ kind: 'error', message: `Recording: ${recorded.problems.join('; ')}` }))
  }
}

// start recording every armed track from the playhead, or stop (playback
// stops too, keeping its place) and put the takes on their tracks
export const toggleRecording = () => async (dispatch: Dispatch, getState: () => RootState) => {
  const command = getState().recording.recording ? 't_stop_recording' : 't_start_recording'
  try {
    receive(dispatch, await invoke<Recorded>(command, {}))
  } catch(e) {
    dispatch(setProjectNotice({ kind: 'error', message: `Couldn't record: ${message(e)}` }))
  }
  await syncPlayingFrom(dispatch).catch(() => {})
}

// while recording: catch it ending some other way (playback stopped, or a
// loop came round), and the takes it left
export const collectRecording = () => async (dispatch: Dispatch) => {
  try {
    receive(dispatch, await invoke<Recorded>('collect_recording', {}))
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not check on the recording:', e)
  }
}

export default recordingSlice.reducer
