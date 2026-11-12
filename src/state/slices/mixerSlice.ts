import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { RootState } from 'state/store'
import {
  ChannelSettings,
  defaultChannelSettings,
  MAX_NAME_CHARS,
  MixerSettings,
  defaultTrackColor,
  trackLabel,
} from '../../types/mixer'
import { LoadedProject } from '../../types/project'
import { reduceLoadedProject, reducePainted, reduceProjectView } from './playlistSlice'
import { currentGesture } from '../gesture'

// the Mixer tab's state. the engine owns the mixer and applies changes while
// playing; this mirrors it so the controls move straight away

export interface MixerState {
  // by track number. tracks not listed are at their defaults
  tracks: Record<number, ChannelSettings>
  master: ChannelSettings
  // names given to tracks, by track number
  names: Record<number, string>
  // colors given to tracks, by track number (see TRACK_COLORS)
  colors: Record<number, number>
}

const initialState: MixerState = {
  tracks: {},
  master: defaultChannelSettings,
  names: {},
  colors: {},
}

const fromSettings = (settings: MixerSettings): MixerState => ({
  tracks: Object.fromEntries(settings.tracks.map(({ trackNumber, name: _, color: __, ...channel }) => [trackNumber, channel])),
  master: settings.master,
  names: Object.fromEntries(settings.tracks
    .filter(track => track.name)
    .map(track => [track.trackNumber, track.name as string])),
  colors: Object.fromEntries(settings.tracks
    .filter(track => track.color !== undefined && track.color !== null)
    .map(track => [track.trackNumber, track.color as number])),
})

export const mixerSlice = createSlice({
  name: 'mixerSlice',
  initialState,
  reducers: {
    setMixer: (_, action: PayloadAction<MixerSettings>) => fromSettings(action.payload),
    setTrack: (state, action: PayloadAction<{ trackNumber: number, change: Partial<ChannelSettings> }>) => {
      const { trackNumber, change } = action.payload
      state.tracks[trackNumber] = { ...(state.tracks[trackNumber] ?? defaultChannelSettings), ...change }
    },
    setMaster: (state, action: PayloadAction<Partial<ChannelSettings>>) => {
      state.master = { ...state.master, ...action.payload }
    },
    setName: (state, action: PayloadAction<{ trackNumber: number, name: string | null }>) => {
      const { trackNumber, name } = action.payload
      if(name) state.names[trackNumber] = name
      else delete state.names[trackNumber]
    },
    setColor: (state, action: PayloadAction<{ trackNumber: number, color: number | null }>) => {
      const { trackNumber, color } = action.payload
      // its default is kept as no color, as the engine keeps it
      if(color === null || color === defaultTrackColor(trackNumber)) delete state.colors[trackNumber]
      else state.colors[trackNumber] = color
    },
  },
  extraReducers: builder => {
    // a project brings its own mix
    builder.addCase(reduceLoadedProject, (state, action: PayloadAction<LoadedProject>) =>
      (action.payload.mixer ? fromSettings(action.payload.mixer) : state))
    // and an undo or redo may have changed it
    builder.addCase(reduceProjectView, (_, action) => fromSettings(action.payload.project.mixer))
    // and a paint below the last track has made a track
    builder.addCase(reducePainted, (_, action) => fromSettings(action.payload.mixer))
  },
})

export const { setMixer, setTrack, setMaster, setName, setColor } = mixerSlice.actions

export const selectTrackMix = (trackNumber: number) => (state: RootState): ChannelSettings =>
  state.mixer.tracks[trackNumber] ?? defaultChannelSettings
export const selectMasterMix = (state: RootState) => state.mixer.master
// a track's name, or "Track n" if it hasn't been given one
export const selectTrackName = (trackNumber: number) => (state: RootState): string =>
  state.mixer.names[trackNumber] ?? trackLabel(trackNumber)
// a track's color (see TRACK_COLORS): the one it was given, or its default
export const selectTrackColor = (trackNumber: number) => (state: RootState): number =>
  state.mixer.colors[trackNumber] ?? defaultTrackColor(trackNumber)
export const selectTrackColors = (state: RootState) => state.mixer.colors
// the names given to tracks (tracks not listed are "Track n")
export const selectTrackNames = (state: RootState) => state.mixer.names
export const selectAnySolo = (state: RootState) =>
  Object.values(state.mixer.tracks).some(track => track.solo)
// every track's mix, by track number (tracks not listed are the defaults)
export const selectTrackMixes = (state: RootState) => state.mixer.tracks

// how often a fader's value is sent while it's being dragged
const SEND_EVERY_MS = 30

// per control: the value waiting to be sent, while a send was recent
const queued = new Map<string, (() => Promise<unknown>) | null>()

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// send a control's value at most every SEND_EVERY_MS. the latest value is
// always sent last
const sendLatest = (key: string, send: () => Promise<unknown>) => {
  if(queued.has(key)) {
    queued.set(key, send)
    return
  }

  send()
  queued.set(key, null)

  const flush = () => {
    const next = queued.get(key)

    if(next) {
      next()
      queued.set(key, null)
      setTimeout(flush, SEND_EVERY_MS)
    } else {
      queued.delete(key)
    }
  }

  setTimeout(flush, SEND_EVERY_MS)
}

// forget values still waiting to be sent, e.g. before a project replaces the
// mix, so they don't overwrite it
export const clearPendingMixerSends = () => queued.clear()

// send the values still waiting, now (before an undo: a value has to have
// reached the engine to be undone)
export const flushMixerSends = (): Promise<unknown> =>
  Promise.all(Array.from(queued.entries()).map(([key, send]) => {
    if(!send) return undefined
    queued.set(key, null)
    return send()
  }))

export const fetchMixer = () => async (dispatch: Dispatch) => {
  const settings = await invoke<MixerSettings>('get_mixer', {})
  dispatch(setMixer(settings))
}

export const setTrackVolume = (trackNumber: number, volumeDb: number) => (dispatch: Dispatch) => {
  dispatch(setTrack({ trackNumber, change: { volumeDb } }))
  // the drag this is part of, which is undone as one step
  const gesture = currentGesture()
  sendLatest(`volume-${trackNumber}`, () =>
    invoke<number>('set_track_volume', { trackNumber, volumeDb, gesture })
      .catch(logError(`set the volume of track ${trackNumber + 1}`)))
}

export const setTrackPan = (trackNumber: number, pan: number) => (dispatch: Dispatch) => {
  dispatch(setTrack({ trackNumber, change: { pan } }))
  const gesture = currentGesture()
  sendLatest(`pan-${trackNumber}`, () =>
    invoke<number>('set_track_pan', { trackNumber, pan, gesture })
      .catch(logError(`set the pan of track ${trackNumber + 1}`)))
}

export const setTrackMute = (trackNumber: number, mute: boolean) => (dispatch: Dispatch) => {
  dispatch(setTrack({ trackNumber, change: { mute } }))
  return invoke<void>('set_track_mute', { trackNumber, mute })
    .catch(logError(`mute track ${trackNumber + 1}`))
}

export const setTrackSolo = (trackNumber: number, solo: boolean) => (dispatch: Dispatch) => {
  dispatch(setTrack({ trackNumber, change: { solo } }))
  return invoke<void>('set_track_solo', { trackNumber, solo })
    .catch(logError(`solo track ${trackNumber + 1}`))
}

export const setMasterVolume = (volumeDb: number) => (dispatch: Dispatch) => {
  dispatch(setMaster({ volumeDb }))
  const gesture = currentGesture()
  sendLatest('volume-master', () =>
    invoke<number>('set_master_volume', { volumeDb, gesture })
      .catch(logError('set the master volume')))
}

// name a track, or clear its name with an empty one
export const setTrackName = (trackNumber: number, name: string) => async (dispatch: Dispatch) => {
  const tidy = name.trim().slice(0, MAX_NAME_CHARS)
  dispatch(setName({ trackNumber, name: tidy || null }))

  try {
    // the engine has the final say on what's kept
    const kept = await invoke<string | null>('set_track_name', { trackNumber, name: tidy })
    dispatch(setName({ trackNumber, name: kept ?? null }))
  } catch(e) {
    logError(`rename track ${trackNumber + 1}`)(e)
  }
}

// give a track a color (see TRACK_COLORS), or null for its default
export const setTrackColor = (trackNumber: number, color: number | null) => async (dispatch: Dispatch) => {
  dispatch(setColor({ trackNumber, color }))

  try {
    const kept = await invoke<number>('set_track_color', { trackNumber, color })
    dispatch(setColor({ trackNumber, color: kept }))
  } catch(e) {
    logError(`color track ${trackNumber + 1}`)(e)
  }
}

// every strip back to its defaults (names and colors are kept)
export const resetMixer = () => async (dispatch: Dispatch) => {
  clearPendingMixerSends()
  const settings = await invoke<MixerSettings>('reset_mixer', {})
  dispatch(setMixer(settings))
}

export default mixerSlice.reducer
