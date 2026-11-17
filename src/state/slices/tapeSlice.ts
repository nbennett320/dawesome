import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { RootState } from 'state/store'
import {
  DEFAULT_DRIVE_DB, defaultTapeMachine, TapeMachine, TapeSettings, TrackTape,
} from '../../types/tape'
import { LoadedProject } from '../../types/project'
import { reduceLoadedProject, reduceProjectView } from './playlistSlice'
import { sendLatest } from './mixerSlice'
import { currentGesture } from '../gesture'

// the project's tape: the switch and the machine's controls (the Transport
// tab), and the tracks on it with their drive (the Mixer tab). the engine
// keeps it with the project, saves it and undoes changes to it, as it does
// the mixer; this mirrors it so the controls move straight away.
//
// nothing here changes the sound yet: the engine only keeps the settings
// (see to_engine in engine/src/daw/actions/tape.rs)

export interface TapeState {
  machine: TapeMachine
  // by track number. tracks not listed are off the tape, at 0 dB
  tracks: Record<number, Omit<TrackTape, 'trackNumber'>>
}

const initialState: TapeState = {
  machine: defaultTapeMachine,
  tracks: {},
}

const defaultTrack = { onTape: false, driveDb: DEFAULT_DRIVE_DB }

// what the engine sent, with anything it left out (an older engine) at its
// default
const fromSettings = (settings: TapeSettings | undefined): TapeState => {
  if(!settings) return initialState
  const { tracks = [], ...machine } = settings
  return {
    machine: { ...defaultTapeMachine, ...machine },
    tracks: Object.fromEntries(tracks.map(({ trackNumber, ...track }) => [trackNumber, track])),
  }
}

export const tapeSlice = createSlice({
  name: 'tapeSlice',
  initialState,
  reducers: {
    setTape: (_, action: PayloadAction<TapeSettings>) => fromSettings(action.payload),
    setMachine: (state, action: PayloadAction<Partial<TapeMachine>>) => {
      state.machine = { ...state.machine, ...action.payload }
    },
    setTrackTape: (state, action: PayloadAction<{ trackNumber: number, change: Partial<Omit<TrackTape, 'trackNumber'>> }>) => {
      const { trackNumber, change } = action.payload
      const track = { ...(state.tracks[trackNumber] ?? defaultTrack), ...change }
      // back at its defaults is as if it was never changed, as the engine keeps it
      if(!track.onTape && track.driveDb === DEFAULT_DRIVE_DB) delete state.tracks[trackNumber]
      else state.tracks[trackNumber] = track
    },
  },
  extraReducers: builder => {
    // a project brings its own tape (an older engine sends none: tape off,
    // no track on it, T14)
    builder.addCase(reduceLoadedProject, (_, action: PayloadAction<LoadedProject>) => fromSettings(action.payload.tape))
    // and an undo or redo may have changed it
    builder.addCase(reduceProjectView, (state, action) =>
      (action.payload.project.tape ? fromSettings(action.payload.project.tape) : state))
  },
})

export const { setTape, setMachine, setTrackTape } = tapeSlice.actions

export const selectTapeMachine = (state: RootState) => state.tape.machine
export const selectTapeEnabled = (state: RootState) => state.tape.machine.enabled
export const selectTrackTape = (trackNumber: number) => (state: RootState) =>
  state.tape.tracks[trackNumber] ?? defaultTrack

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// the hook points. each change goes to the engine, which keeps it with the
// project (saved, undoable) and is where the sound will follow it (see
// to_engine in engine/src/daw/actions/tape.rs). knob drags send at most
// every few ms, the latest value last, as the mixer's do

// the switch and the machine's controls, all of them
export const sendTapeMachine = (machine: TapeMachine, gesture: number | null) =>
  invoke<TapeMachine>('set_tape_machine', { machine, gesture })

// a track's place on the tape and its drive
export const sendTrackTape = (track: TrackTape, gesture: number | null) =>
  invoke<TrackTape>('set_track_tape', { track, gesture })

export const fetchTape = () => async (dispatch: Dispatch) => {
  const settings = await invoke<TapeSettings>('get_tape', {})
  dispatch(setTape(settings))
}

// change some of the machine's controls (or the switch)
export const changeTapeMachine = (change: Partial<TapeMachine>) =>
  (dispatch: Dispatch, getState: () => RootState) => {
    dispatch(setMachine(change))
    const { machine } = getState().tape
    const gesture = currentGesture()
    sendLatest('tape-machine', () => sendTapeMachine(machine, gesture)
      .catch(logError('change the tape machine')))
  }

// put a track on tape or take it off, or change its drive
export const changeTrackTape = (trackNumber: number, change: Partial<Omit<TrackTape, 'trackNumber'>>) =>
  (dispatch: Dispatch, getState: () => RootState) => {
    dispatch(setTrackTape({ trackNumber, change }))
    const track = { trackNumber, ...selectTrackTape(trackNumber)(getState()) }
    const gesture = currentGesture()
    sendLatest(`tape-${trackNumber}`, () => sendTrackTape(track, gesture)
      .catch(logError(`change track ${trackNumber + 1}'s tape`)))
  }

export default tapeSlice.reducer
