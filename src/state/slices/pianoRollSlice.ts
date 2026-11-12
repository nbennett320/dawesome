import { createSlice, PayloadAction } from '@reduxjs/toolkit'
import type { RootState } from '../store'
import { SnapNote } from '../../editing/snap'
import { Tool } from '../../editing/tools'

// the Piano Roll's mouse tool and snap setting, and each pattern's start
// marker. the page's own: not saved with the project, and kept when another
// one is opened
export interface PianoRollState {
  tool: Tool
  snapEnabled: boolean
  snapNote: SnapNote
  // the step Play starts from in each pattern (set on the ruler), by
  // pattern id. a pattern with none starts from its start
  startSteps: Record<number, number>
}

const initialState: PianoRollState = {
  tool: 'draw',
  snapEnabled: true,
  snapNote: 'SixteenthNote',
  startSteps: {},
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
    setPianoRollStart: (state, action: PayloadAction<{ patternId: number, step: number }>) => {
      state.startSteps[action.payload.patternId] = action.payload.step
    },
  },
})

export const {
  setPianoRollTool, togglePianoRollSnap, setPianoRollSnapNote, setPianoRollStart,
} = pianoRollSlice.actions

export const selectPianoRollTool = (state: RootState) => state.pianoRoll.tool
export const selectPianoRollSnapEnabled = (state: RootState) => state.pianoRoll.snapEnabled
export const selectPianoRollSnapNote = (state: RootState) => state.pianoRoll.snapNote
// the step Play starts a pattern from in the Piano Roll
export const selectPianoRollStart = (patternId: number) => (state: RootState): number =>
  state.pianoRoll.startSteps[patternId] ?? 0

export default pianoRollSlice.reducer
