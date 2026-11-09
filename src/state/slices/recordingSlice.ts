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
  // whether recording counts in a bar first
  countIn: boolean
  // input choices, by track number
  armed: Record<number, InputChoice>
  // the input's channels, or null when there's no input
  inputChannels: number | null
  inputError: string | null
  // goes up whenever the takes in the recordings folder may have changed
  // (recorded, moved by a save, or another project opened), so the
  // recordings browser lists them again
  takesRevision: number
}

const initialState: RecordingState = {
  recording: false,
  countIn: false,
  armed: {},
  inputChannels: null,
  inputError: null,
  takesRevision: 0,
}

export const recordingSlice = createSlice({
  name: 'recordingSlice',
  initialState,
  reducers: {
    setRecording: (state, action: PayloadAction<boolean>) => {
      // starting makes a take's file, stopping finishes it
      if(state.recording !== action.payload) state.takesRevision += 1
      state.recording = action.payload
    },
    takesChanged: state => {
      state.takesRevision += 1
    },
    setCountIn: (state, action: PayloadAction<boolean>) => {
      state.countIn = action.payload
    },
    setRecordSetup: (state, action: PayloadAction<RecordSetup>) => {
      const { armed, inputChannels, inputError } = action.payload
      state.armed = Object.fromEntries(armed.map(track => [track.trackNumber, track.input]))
      state.inputChannels = inputChannels
      state.inputError = inputError
    },
  },
  extraReducers: builder => {
    // a project brings its own armed tracks, and whether to count in
    builder.addCase(reduceLoadedProject, (state, action: PayloadAction<LoadedProject>) => {
      state.countIn = action.payload.countIn ?? false
      state.armed = Object.fromEntries((action.payload.armed ?? []).map(track => [track.trackNumber, track.input]))
      state.takesRevision += 1
    })
    // a save can move takes into the project's folder
    builder.addCase(playlistSlice.actions.reduceMovedClips, state => {
      state.takesRevision += 1
    })
  },
})

export const { setRecording, setRecordSetup, setCountIn, takesChanged } = recordingSlice.actions

export const selectRecording = (state: RootState) => state.recording.recording
export const selectArmed = (trackNumber: number) => (state: RootState): InputChoice | null =>
  state.recording.armed[trackNumber] ?? null
export const selectAnyArmed = (state: RootState) => Object.keys(state.recording.armed).length > 0
export const selectInputChannels = (state: RootState) => state.recording.inputChannels
export const selectCountIn = (state: RootState) => state.recording.countIn
export const selectTakesRevision = (state: RootState) => state.recording.takesRevision

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// the armed tracks, and the input's channels to choose from
export const fetchRecordSetup = () => async (dispatch: Dispatch) => {
  dispatch(setRecordSetup(await invoke<RecordSetup>('get_record_setup')))
}

// arm a track to record these input channels, or (null) disarm it
export const setTrackArmed = (trackNumber: number, input: InputChoice | null) => async (dispatch: Dispatch) => {
  dispatch(setRecordSetup(await invoke<RecordSetup>('set_track_armed', { trackNumber, input })))
}

// count in a bar before recording, or not
export const toggleCountIn = () => async (dispatch: Dispatch) => {
  try {
    dispatch(setCountIn(await invoke<boolean>('t_toggle_count_in', {})))
  } catch(e) {
    // eslint-disable-next-line no-console
    console.error('could not change the count-in:', e)
  }
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
  // a loop's earlier passes land while it's still recording
  if(recorded.clips.length > 0) dispatch(recordingSlice.actions.takesChanged())
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
