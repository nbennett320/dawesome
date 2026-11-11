import { createSlice, PayloadAction } from '@reduxjs/toolkit'
import type { RootState } from '../store'
import { SnapNote } from '../../editing/snap'
import { Tool } from '../../editing/tools'

// the Piano Roll's mouse tool and snap setting. the page's own: not saved
// with the project, and kept when another one is opened
export interface PianoRollState {
  tool: Tool
  snapEnabled: boolean
  snapNote: SnapNote
}

const initialState: PianoRollState = {
  tool: 'draw',
  snapEnabled: true,
  snapNote: 'SixteenthNote',
}

export const pianoRollSlice = createSlice({
  name: 'pianoRoll',
  initialState,
  reducers: {
    setPianoRollTool: (state, action: PayloadAction<Tool>) => {
      state.tool = action.payload
    },
    togglePianoRollSnap: state => {
      state.snapEnabled = !state.snapEnabled
    },
    setPianoRollSnapNote: (state, action: PayloadAction<SnapNote>) => {
      state.snapNote = action.payload
    },
  },
})

export const { setPianoRollTool, togglePianoRollSnap, setPianoRollSnapNote } = pianoRollSlice.actions

export const selectPianoRollTool = (state: RootState) => state.pianoRoll.tool
export const selectPianoRollSnapEnabled = (state: RootState) => state.pianoRoll.snapEnabled
export const selectPianoRollSnapNote = (state: RootState) => state.pianoRoll.snapNote

export default pianoRollSlice.reducer
