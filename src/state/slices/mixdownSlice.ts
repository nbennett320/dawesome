import { createSlice, PayloadAction } from '@reduxjs/toolkit'
import type { RootState } from '../store'

// the Mixdown tab's choices (see engine/src/mixdown_commands.rs), kept while
// the app is open so closing the tab doesn't lose them. not saved with the
// project
export type FileType = 'wav' | 'mp3'
export type Bits = 'int16' | 'int24' | 'float32'
export const KBPS = [128, 192, 256, 320] as const
export type Kbps = typeof KBPS[number]

export const BITS_LABELS: Record<Bits, string> = {
  int16: '16-bit',
  int24: '24-bit',
  float32: '32-bit float',
}

export interface MixdownState {
  fileType: FileType
  bits: Bits
  kbps: Kbps
  // the name and folder typed or chosen, or null for the project's. they
  // belong to the project that was open when they were chosen
  name: string | null
  dir: string | null
  forProject: string | null
  // a mixdown was started from the tab, and its outcome hasn't been put
  // away yet (with New mixdown): the tab shows how it's going, or went
  watching: boolean
}

const initialState: MixdownState = {
  fileType: 'wav',
  bits: 'int16',
  kbps: 320,
  name: null,
  dir: null,
  forProject: null,
  watching: false,
}

export const mixdownSlice = createSlice({
  name: 'mixdownSlice',
  initialState,
  reducers: {
    setFileType: (state, action: PayloadAction<FileType>) => { state.fileType = action.payload },
    setBits: (state, action: PayloadAction<Bits>) => { state.bits = action.payload },
    setKbps: (state, action: PayloadAction<Kbps>) => { state.kbps = action.payload },
    // the name or folder chosen, with the project open when it was
    setMixdownPlace: (state, action: PayloadAction<{ project: string | null, name?: string, dir?: string }>) => {
      const { project, name, dir } = action.payload
      if(state.forProject !== project) {
        state.name = null
        state.dir = null
        state.forProject = project
      }
      if(name !== undefined) state.name = name
      if(dir !== undefined) state.dir = dir
    },
    setWatching: (state, action: PayloadAction<boolean>) => { state.watching = action.payload },
  },
})

export const { setFileType, setBits, setKbps, setMixdownPlace, setWatching } = mixdownSlice.actions

// the name and folder chosen for this project, or null where the project's
// own are to be used
export const chosenPlace = (state: MixdownState, project: string | null) =>
  state.forProject === project ? { name: state.name, dir: state.dir } : { name: null, dir: null }

// what the engine is sent as the file's format
export const formatOf = (state: MixdownState) =>
  state.fileType === 'wav' ? { type: 'wav', bits: state.bits } : { type: 'mp3', kbps: state.kbps }

export const selectMixdown = (state: RootState) => state.mixdown

export default mixdownSlice.reducer
