import { createSelector, createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { RootState } from '../store'
import { Note, Pattern, SequencerState, withStep } from '../../types/sequencer'
import { instrumentTracks } from '../../types/effects'
import { PlaylistTransport } from '../../types/transport'
import { View } from '../../types/ui'
import { LoadedPatternClip } from '../../types/project'
import { MixerSettings } from '../../types/mixer'
import { setMixer } from './mixerSlice'
import {
  playlistSlice,
  reduceLoadedProject,
  refreshTrackCount,
  removePlaylistItem,
  setPlaying,
  syncPlayingFrom,
  togglePlay,
  setPlaylistTempo,
  reduceProjectView,
} from './playlistSlice'
import { openTab, selectFocusedView } from './windowSlice'
import { currentGesture } from '../gesture'

// a pattern playing on its own, and when it started, for the step highlight
export interface PatternRun {
  patternId: number
  // ms since the unix epoch, like Date.now()
  startTimeMs: number
  beatIntervalMs: number
}

export interface SequencerSliceState extends SequencerState {
  // the pattern the Sequencer and Piano Roll tabs show, which their Play
  // loops
  selectedPatternId: number | null
  run: PatternRun | null
  // the track whose instrument plays a pattern's notes while it plays on
  // its own (the Piano Roll's "Plays on"), as picked. null, or a track that
  // has lost its instrument, means the first track with one (see
  // selectPlaysOn)
  playsOn: number | null
}

const initialState: SequencerSliceState = {
  rows: [],
  patterns: [],
  folders: [],
  selectedPatternId: null,
  run: null,
  playsOn: null,
}

// keep a pattern selected while there are any: the one that was, or the
// first
const fixSelection = (state: SequencerSliceState) => {
  if(state.patterns.some(p => p.id === state.selectedPatternId)) return
  state.selectedPatternId = state.patterns[0]?.id ?? null
}

export const sequencerSlice = createSlice({
  name: 'sequencerSlice',
  initialState,
  reducers: {
    // the sequencer as the engine has it, after any edit
    setSequencer: (state, action: PayloadAction<SequencerState>) => {
      state.rows = action.payload.rows
      state.patterns = action.payload.patterns
      state.folders = action.payload.folders ?? []
      fixSelection(state)
    },
    selectPattern: (state, action: PayloadAction<number>) => {
      if(state.patterns.some(p => p.id === action.payload)) state.selectedPatternId = action.payload
    },
    // show a step changed straight away, before the engine answers, so
    // painting across a row keeps up with the mouse
    reduceStep: (
      state,
      action: PayloadAction<{ patternId: number, rowId: number, step: number, on: boolean }>,
    ) => {
      const { patternId, rowId, step, on } = action.payload
      const i = state.patterns.findIndex(p => p.id === patternId)
      if(i >= 0) state.patterns[i] = withStep(state.patterns[i], rowId, step, on)
    },
    // and a pattern's notes, as they're drawn and dragged
    reduceNotes: (state, action: PayloadAction<{ patternId: number, notes: Note[] }>) => {
      const pattern = state.patterns.find(p => p.id === action.payload.patternId)
      if(pattern) pattern.notes = action.payload.notes
    },
    setRun: (state, action: PayloadAction<PatternRun | null>) => {
      state.run = action.payload
    },
    setPlaysOn: (state, action: PayloadAction<number | null>) => {
      state.playsOn = action.payload
    },
  },
  extraReducers: builder => {
    builder
      .addCase(reduceLoadedProject, (state, action) => {
        const { sequencer } = action.payload
        state.rows = sequencer?.rows ?? []
        state.patterns = sequencer?.patterns ?? []
        state.folders = sequencer?.folders ?? []
        state.selectedPatternId = null
        state.run = null
        state.playsOn = null
        fixSelection(state)
      })
      // an undo or redo may have changed the rows and patterns. the selected
      // pattern stays selected while it's there
      .addCase(reduceProjectView, (state, action) => {
        const { sequencer } = action.payload.project
        state.rows = sequencer.rows
        state.patterns = sequencer.patterns
        state.folders = sequencer.folders
        fixSelection(state)
      })
      // nothing plays once the engine has stopped
      .addCase(setPlaying, (state, action) => {
        if(!action.payload) state.run = null
      })
  },
})

export const { selectPattern, setSequencer, setPlaysOn } = sequencerSlice.actions
const { reduceStep, reduceNotes, setRun } = sequencerSlice.actions

export default sequencerSlice.reducer

const sequencerError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// run a sequencer command that returns the whole sequencer, and keep it
const edit = (what: string, command: string, args: Record<string, unknown>) =>
  (dispatch: Dispatch) =>
    invoke<SequencerState>(command, args)
      .then(sequencer => { dispatch(setSequencer(sequencer)) })
      .catch(sequencerError(what))

// the sequencer as the engine has it, e.g. when the tab first shows
export const loadSequencer = () => (dispatch: Dispatch) =>
  invoke<SequencerState>('get_sequencer', {})
    .then(sequencer => { dispatch(setSequencer(sequencer)) })
    .catch(sequencerError('load the sequencer'))

// a new empty pattern at the end, selected
export const addPattern = (bars: number) => (dispatch: Dispatch) =>
  invoke<{ id: number, sequencer: SequencerState }>('add_pattern', { bars })
    .then(({ id, sequencer }) => {
      dispatch(setSequencer(sequencer))
      dispatch(selectPattern(id))
    })
    .catch(sequencerError('add a pattern'))

// a sample dropped on the sequencer becomes a row. a project needs a
// pattern to put steps in, so the first row brings one along
export const addSequencerRow = (path: string) => async (dispatch: Dispatch, getState: () => RootState) => {
  await edit(`add ${path} to the sequencer`, 'add_sequencer_row', { path })(dispatch)
  if(!getState().sequencer.patterns.length) await addPattern(1)(dispatch)
}

export const removeSequencerRow = (rowId: number) =>
  edit('remove the row', 'remove_sequencer_row', { rowId })

// (one drag of the knob is one step to undo: see state/gesture.ts)
export const setSequencerRowVolume = (rowId: number, volume: number) =>
  edit("set the row's volume", 'set_sequencer_row_volume', { rowId, volume, gesture: currentGesture() })

export const setSequencerRowMuted = (rowId: number, muted: boolean) =>
  edit('mute the row', 'set_sequencer_row_muted', { rowId, muted })

export const previewSequencerRow = (rowId: number) => () =>
  invoke('preview_sequencer_row', { rowId }).catch(sequencerError('preview the row'))

// a copy of a pattern straight after it, selected
export const duplicatePattern = (patternId: number) => (dispatch: Dispatch) =>
  invoke<{ id: number, sequencer: SequencerState }>('duplicate_pattern', { patternId })
    .then(({ id, sequencer }) => {
      dispatch(setSequencer(sequencer))
      dispatch(selectPattern(id))
    })
    .catch(sequencerError('duplicate the pattern'))

// delete a pattern, and its clips in the playlist
export const removePattern = (patternId: number) => async (dispatch: Dispatch) => {
  try {
    const { removedClips, sequencer } = await invoke<{ removedClips: number[], sequencer: SequencerState }>(
      'remove_pattern', { patternId })
    dispatch(setSequencer(sequencer))
    removedClips.forEach(id => dispatch(removePlaylistItem({ id })))
    if(removedClips.length) await refreshTrackCount()(dispatch)
  } catch(e) {
    sequencerError('delete the pattern')(e)
  }
}

// pattern folders, in the sidebar's pattern browser. unlike the edits above,
// these throw, so the browser can say why (a name that's taken, a folder
// whose patterns are in the playlist)
const folderEdit = (command: string, args: Record<string, unknown>) => async (dispatch: Dispatch) => {
  dispatch(setSequencer(await invoke<SequencerState>(command, args)))
}

// a new, empty folder in `parent` (null: the top). returns its id
export const addPatternFolder = (name: string, parent: number | null) =>
  async (dispatch: Dispatch): Promise<number> => {
    const { id, sequencer } = await invoke<{ id: number, sequencer: SequencerState }>(
      'add_pattern_folder', { name, parent })
    dispatch(setSequencer(sequencer))
    return id
  }

export const renamePatternFolder = (folderId: number, name: string) =>
  folderEdit('rename_pattern_folder', { folderId, name })

// move a folder into another, or (null) to the top
export const movePatternFolder = (folderId: number, parent: number | null) =>
  folderEdit('move_pattern_folder', { folderId, parent })

// put a pattern in a folder, or (null) at the top
export const movePatternToFolder = (patternId: number, folder: number | null) =>
  folderEdit('move_pattern', { patternId, folder })

// delete a folder, its folders and its patterns. the engine refuses while
// any of them is in the playlist
export const removePatternFolder = (folderId: number) =>
  folderEdit('remove_pattern_folder', { folderId })

export const renamePattern = (patternId: number, name: string) =>
  edit('rename the pattern', 'rename_pattern', { patternId, name })

export const setPatternBars = (patternId: number, bars: number) =>
  edit("change the pattern's length", 'set_pattern_bars', { patternId, bars })

export const setPatternStep = (patternId: number, rowId: number, step: number, on: boolean) =>
  (dispatch: Dispatch) => {
    dispatch(reduceStep({ patternId, rowId, step, on }))
    // every step painted in one stroke of the mouse is undone together
    const gesture = currentGesture()
    return edit('change the step', 'set_pattern_step', { patternId, rowId, step, on, gesture })(dispatch)
  }

// how often a pattern's notes are sent while they're being dragged
const SEND_NOTES_EVERY_MS = 30

// per pattern: the notes waiting to be sent, while a send was recent
interface QueuedNotes {
  notes: Note[]
  // the gesture they were changed in, taken when they were (it may be over
  // by the time they're sent)
  gesture: number | null
  dispatch: Dispatch
}
const queuedNotes = new Map<number, QueuedNotes | null>()

// counts the sends, so only the answer to the latest is kept: an earlier
// one arriving late would put back notes since dragged elsewhere. nor is an
// answer kept while newer notes are waiting to be sent
let noteSends = 0

const sendNotes = (patternId: number, { notes, gesture, dispatch }: QueuedNotes): Promise<void> => {
  noteSends += 1
  const send = noteSends
  return invoke<SequencerState>('set_pattern_notes', { patternId, notes, gesture })
    .then(sequencer => {
      if(send === noteSends && !queuedNotes.get(patternId)) dispatch(setSequencer(sequencer))
    })
    .catch(e => {
      sequencerError('change the notes')(e)
      // and show the notes as the engine has them
      return loadSequencer()(dispatch)
    })
}

// replace a pattern's notes. shown straight away, and sent at most every
// SEND_NOTES_EVERY_MS, the latest always last (flushNoteSends sends it
// now). every change made in one drag is one step to undo: see
// state/gesture.ts
export const setPatternNotes = (patternId: number, notes: Note[]) => (dispatch: Dispatch): Promise<void> => {
  dispatch(reduceNotes({ patternId, notes }))
  const change = { notes, gesture: currentGesture(), dispatch }

  if(queuedNotes.has(patternId)) {
    queuedNotes.set(patternId, change)
    return Promise.resolve()
  }

  queuedNotes.set(patternId, null)
  const flush = () => {
    const next = queuedNotes.get(patternId)
    if(next) {
      queuedNotes.set(patternId, null)
      sendNotes(patternId, next)
      setTimeout(flush, SEND_NOTES_EVERY_MS)
    } else {
      queuedNotes.delete(patternId)
    }
  }
  setTimeout(flush, SEND_NOTES_EVERY_MS)

  return sendNotes(patternId, change)
}

// send the notes still waiting, now: when a drag ends, and before an undo
// (a change has to have reached the engine to be undone)
export const flushNoteSends = (): Promise<unknown> =>
  Promise.all(Array.from(queuedNotes.entries()).map(([patternId, next]) => {
    if(!next) return undefined
    queuedNotes.set(patternId, null)
    return sendNotes(patternId, next)
  }))

// forget notes still waiting to be sent, and answers still to come, e.g.
// after an undo has replaced them
export const clearPendingNoteSends = () => {
  queuedNotes.clear()
  noteSends += 1
}

// the tabs that show the selected pattern, and loop it with Play while
// they're the focused pane's
export const PATTERN_VIEWS = [View.Sequencer, View.PianoRoll]

export const isPatternView = (view: View | undefined): boolean =>
  view !== undefined && PATTERN_VIEWS.includes(view)

// the tracks with an instrument to play notes on, in order
export const selectInstrumentTracks = createSelector(
  [(state: RootState) => state.effects.racks],
  racks => instrumentTracks(racks),
)

// the track whose instrument plays a pattern's notes while it plays on its
// own: the one picked, while it has an instrument, or else the first track
// with one. null when no track has one (its notes aren't heard)
export const selectPlaysOn = (state: RootState): number | null => {
  const tracks = selectInstrumentTracks(state)
  const picked = state.sequencer.playsOn
  if(picked !== null && tracks.includes(picked)) return picked
  return tracks[0] ?? null
}

// after a transport command: whether the engine plays, and which pattern
export const syncRun = async (dispatch: Dispatch) => {
  await syncPlayingFrom(dispatch)
  const t = await invoke<PlaylistTransport>('get_playlist_transport', {})
  dispatch(setRun(t.playing && t.patternId !== null && t.startTimeMs !== null
    ? { patternId: t.patternId, startTimeMs: t.startTimeMs, beatIntervalMs: t.beatIntervalMs }
    : null))
}

// change the tempo (see setPlaylistTempo). a pattern playing on its own
// starts over at the new tempo, so the Sequencer's step light follows the
// new run. resolves to whether the tempo was changed
export const changeTempo = (tempo: number, gesture: number | null = null) => async (
  dispatch: Dispatch,
): Promise<boolean> => {
  const changed = await setPlaylistTempo(tempo, gesture)(dispatch)
  if(changed) await syncRun(dispatch)
  return changed
}

// split pattern clips in the playlist into a pattern clip per sample, each
// playing a pattern of just that sample's steps (made in the pattern list, or
// reused from an earlier explode) on its own track, so its own mixer strip.
// ids that aren't pattern clips are skipped
export const explodePatternClips = (ids: number[]) => async (dispatch: Dispatch) => {
  if(!ids.length) return
  try {
    const { removed, clips, sequencer, mixer } = await invoke<{
      removed: number[], clips: LoadedPatternClip[], sequencer: SequencerState, mixer: MixerSettings,
    }>('explode_pattern_clips', { ids })
    dispatch(setSequencer(sequencer))
    dispatch(playlistSlice.actions.reduceExploded({ removed, clips }))
    dispatch(setMixer(mixer))
    await refreshTrackCount()(dispatch)
  } catch(e) {
    sequencerError('explode the pattern')(e)
  }
}

// open a pattern in the Sequencer tab (double-clicking its clip)
export const showPattern = (patternId: number) => (dispatch: Dispatch) => {
  dispatch(selectPattern(patternId))
  dispatch(openTab(View.Sequencer))
}

// loop a pattern on its own, its notes on the "Plays on" track's
// instrument, or stop it (or pause what's playing)
export const togglePattern = (patternId: number) => async (dispatch: Dispatch, getState: () => RootState) => {
  try {
    await invoke('t_toggle_pattern', { patternId, track: selectPlaysOn(getState()) })
  } catch(e) {
    sequencerError('play the pattern')(e)
  }
  await syncRun(dispatch)
}

// the Play key: with the Sequencer or the Piano Roll the focused pane's
// tab, loop its pattern (or pause what's playing); anywhere else, play the
// playlist as always
export const togglePlayHere = () => async (dispatch: Dispatch, getState: () => RootState) => {
  const state = getState()
  const patternId = state.sequencer.selectedPatternId

  if(!isPatternView(selectFocusedView(state)) || patternId === null) {
    await togglePlay()(dispatch)
    dispatch(setRun(null))
    return
  }

  await togglePattern(patternId)(dispatch, getState)
}

// loop a pattern from its start, stopping whatever was playing
export const playPattern = (patternId: number) => async (dispatch: Dispatch, getState: () => RootState) => {
  try {
    await invoke('t_play_pattern', { patternId, track: selectPlaysOn(getState()) })
  } catch(e) {
    sequencerError('play the pattern')(e)
  }
  await syncRun(dispatch)
}

// stop a pattern playing on its own (focus left the Sequencer and the
// Piano Roll). the
// playlist's playhead stays where it was
export const stopPattern = () => async (dispatch: Dispatch) => {
  try {
    await invoke('t_pause_playlist', {})
  } catch(e) {
    sequencerError('stop the pattern')(e)
  }
  await syncRun(dispatch)
}

export const selectSequencerRows = (state: RootState) => state.sequencer.rows
export const selectPatterns = (state: RootState) => state.sequencer.patterns
export const selectPatternFolders = (state: RootState) => state.sequencer.folders
export const selectSelectedPatternId = (state: RootState) => state.sequencer.selectedPatternId
export const selectSelectedPattern = (state: RootState): Pattern | null =>
  state.sequencer.patterns.find(p => p.id === state.sequencer.selectedPatternId) ?? null
export const selectPatternRun = (state: RootState) => state.sequencer.run
