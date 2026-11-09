import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import type { RootState } from '../store'
import { RecordingStatus, takeSummary } from '../../types/audio'
import { setProjectNotice } from './projectSlice'

// whether a take is being recorded (see t_toggle_recording in
// engine/src/audio_commands.rs)
export interface RecordingState {
  recording: boolean
}

const initialState: RecordingState = {
  recording: false,
}

export const recordingSlice = createSlice({
  name: 'recordingSlice',
  initialState,
  reducers: {
    setRecording: (state, action: PayloadAction<boolean>) => {
      state.recording = action.payload
    },
  },
})

export const { setRecording } = recordingSlice.actions

export const selectRecording = (state: RootState) => state.recording.recording

// start recording a take, or stop and keep it, saying where it went
export const toggleRecording = () => async (dispatch: Dispatch) => {
  try {
    const status = await invoke<RecordingStatus>('t_toggle_recording', {})
    dispatch(setRecording(status.recording))
    if(status.take) dispatch(setProjectNotice({ kind: 'info', message: takeSummary(status.take) }))
  } catch(e) {
    dispatch(setProjectNotice({ kind: 'error', message: `Couldn't record: ${e instanceof Error ? e.message : String(e)}` }))
  }
}

export default recordingSlice.reducer
