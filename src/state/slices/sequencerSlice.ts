import { createSelector, createSlice, Dispatch, Draft, original, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { RootState } from '../store'
import {
  AddedRow, Note, Pattern, PatternChange, PatternFolder, Row, RowChange, SequencerState, SynthInstrument,
  isSampleRow, isSynthRow, keepNotes, keepUnchanged, samePattern, sameRow, sortNotes, withNotes, withStep,
} from '../../types/sequencer'
import { PlaylistTransport } from '../../types/transport'
import { View } from '../../types/ui'
import { LoadedPatternClip, ProjectView } from '../../types/project'
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
  reducePainted,
} from './playlistSlice'
import { setProjectNotice } from './projectSlice'
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
  // the pattern shown in the sequencers and the Piano Roll, which Play
  // loops while one of them is focused
  selected: number | null
  // the pattern most recently selected or made in a sequencer: what the
  // playlist's Draw tool paints. null when there isn't one (it was deleted,
  // or a project was opened since)
  lastSelected: number | null
  // the synth row whose notes the Piano Roll edits
  pianoRollRowId: number | null
  run: PatternRun | null
}

const initialState: SequencerSliceState = {
  rows: [],
  patterns: [],
  folders: [],
  selected: null,
  lastSelected: null,
  pianoRollRowId: null,
  run: null,
}

// keep a pattern selected while there are any: the one that was, or the
// first. and a synth row for the Piano Roll while there are any
const fixSelection = (state: SequencerSliceState) => {
  if(!state.patterns.some(p => p.id === state.selected)) state.selected = state.patterns[0]?.id ?? null
  // (not replaced by another: the Draw tool makes a pattern instead)
  if(!state.patterns.some(p => p.id === state.lastSelected)) state.lastSelected = null
  if(!state.rows.some(row => row.id === state.pianoRollRowId && isSynthRow(row))) {
    state.pianoRollRowId = state.rows.find(isSynthRow)?.id ?? null
  }
}

const sameFolders = (a: PatternFolder[], b: PatternFolder[]): boolean =>
  a.length === b.length && a.every((folder, i) => folder.id === b[i].id && folder.name === b[i].name
    && (folder.parent ?? null) === (b[i].parent ?? null))

// the whole sequencer as the engine sent it, keeping every row and pattern
// it didn't change (by rev: see keepUnchanged), and the folders when none did.
// (what's kept is taken from the state before this action, with `original`,
// so it's the very object selectors and memoised rows saw, not a draft)
const take = (state: Draft<SequencerSliceState>, sequencer: SequencerState | null | undefined) => {
  state.rows = keepUnchanged(original(state.rows), sequencer?.rows ?? [], sameRow)
  state.patterns = keepUnchanged(original(state.patterns), sequencer?.patterns ?? [], samePattern)
  const folders = sequencer?.folders ?? []
  if(!sameFolders(original(state.folders), folders)) state.folders = folders
}

const select = (state: SequencerSliceState, patternId: number) => {
  if(!state.patterns.some(p => p.id === patternId)) return
  state.selected = patternId
  state.lastSelected = patternId
}

export const sequencerSlice = createSlice({
  name: 'sequencerSlice',
  initialState,
  reducers: {
    // the sequencer as the engine has it, after any edit that answers with
    // all of it
    setSequencer: (state, action: PayloadAction<SequencerState>) => {
      take(state, action.payload)
      fixSelection(state)
    },
    // the one pattern an edit changed, as the engine has it. the rest of the
    // sequencer is kept as it is, and so are the note lists of the rows the
    // edit didn't change (see keepNotes). a pattern deleted meanwhile stays
    // deleted
    reducePattern: (state, action: PayloadAction<PatternChange>) => {
      const got = action.payload.pattern
      const patterns = original(state.patterns)
      const i = patterns.findIndex(p => p.id === got.id)
      if(i >= 0 && !samePattern(patterns[i], got)) state.patterns[i] = keepNotes(patterns[i], got)
    },
    // the one row whose own settings an edit changed, as the engine has it
    reduceRow: (state, action: PayloadAction<RowChange>) => {
      const got = action.payload.row
      const rows = original(state.rows)
      const i = rows.findIndex(row => row.id === got.id)
      if(i >= 0 && !sameRow(rows[i], got)) state.rows[i] = got
    },
    // show a pattern in the sequencers and the Piano Roll
    selectPattern: (state, action: PayloadAction<number>) => {
      select(state, action.payload)
    },
    // the synth row whose notes the Piano Roll edits
    setPianoRollRow: (state, action: PayloadAction<number>) => {
      if(state.rows.some(row => row.id === action.payload && isSynthRow(row))) state.pianoRollRowId = action.payload
    },
    // show a step changed straight away, before the engine answers, so
    // painting across a row keeps up with the mouse
    reduceStep: (
      state,
      action: PayloadAction<{ patternId: number, rowId: number, step: number, on: boolean }>,
    ) => {
      const { patternId, rowId, step, on } = action.payload
      const row = original(state.rows).find(r => r.id === rowId)
      const patterns = original(state.patterns)
      const i = patterns.findIndex(p => p.id === patternId)
      if(row && i >= 0) state.patterns[i] = withStep(patterns[i], row, step, on)
    },
    // and a row's notes in a pattern (in order), as they're drawn and dragged
    reduceNotes: (state, action: PayloadAction<{ patternId: number, rowId: number, notes: Note[] }>) => {
      const { patternId, rowId, notes } = action.payload
      const patterns = original(state.patterns)
      const i = patterns.findIndex(p => p.id === patternId)
      if(i >= 0) state.patterns[i] = withNotes(patterns[i], rowId, notes)
    },
    // and a row moved, as soon as it's dropped
    reduceRowMove: (state, action: PayloadAction<{ rowId: number, index: number }>) => {
      const from = state.rows.findIndex(r => r.id === action.payload.rowId)
      if(from < 0) return
      const [row] = state.rows.splice(from, 1)
      state.rows.splice(Math.min(Math.max(0, action.payload.index), state.rows.length), 0, row)
    },
    setRun: (state, action: PayloadAction<PatternRun | null>) => {
      state.run = action.payload
    },
  },
  extraReducers: builder => {
    builder
      .addCase(reduceLoadedProject, (state, action) => {
        take(state, action.payload.sequencer)
        state.selected = null
        state.lastSelected = null
        state.pianoRollRowId = null
        state.run = null
        fixSelection(state)
      })
      // an undo or redo may have changed the rows and patterns. the
      // selected ones stay selected while they're there
      .addCase(reduceProjectView, (state, action) => {
        take(state, action.payload.project.sequencer)
        fixSelection(state)
      })
      // clips painted with the playlist's Draw tool: the sequencer, with the
      // pattern they play selected (made by the paint, when there was none)
      .addCase(reducePainted, (state, action) => {
        take(state, action.payload.sequencer)
        fixSelection(state)
        select(state, action.payload.patternId)
      })
      // nothing plays once the engine has stopped
      .addCase(setPlaying, (state, action) => {
        if(!action.payload) state.run = null
      })
  },
})

export const { selectPattern, setPianoRollRow, setSequencer } = sequencerSlice.actions
const { reducePattern, reduceRow, reduceStep, reduceNotes, reduceRowMove, setRun } = sequencerSlice.actions

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

// run a command that changes one row's own settings, and keep the row it
// returns
const editRow = (what: string, command: string, args: Record<string, unknown>) =>
  (dispatch: Dispatch) =>
    invoke<RowChange>(command, args)
      .then(change => { dispatch(reduceRow(change)) })
      .catch(sequencerError(what))

// run a command that changes one pattern, and keep the pattern it returns.
// if it's refused, the sequencer is fetched again, so a change shown before
// the engine answered (see setPatternStep) doesn't stay
const editPattern = (what: string, command: string, args: Record<string, unknown>) =>
  (dispatch: Dispatch) =>
    invoke<PatternChange>(command, args)
      .then(change => { dispatch(reducePattern(change)) })
      .catch(e => {
        sequencerError(what)(e)
        return loadSequencer()(dispatch)
      })

// a new empty pattern at the end, selected
export const addPattern = (bars: number) => (dispatch: Dispatch) =>
  invoke<{ id: number, sequencer: SequencerState }>('add_pattern', { bars })
    .then(({ id, sequencer }) => {
      dispatch(setSequencer(sequencer))
      dispatch(selectPattern(id))
    })
    .catch(sequencerError('add a pattern'))

// a project needs a pattern to put notes in, so the first row brings one
// along
const addPatternIfNone = async (dispatch: Dispatch, getState: () => RootState) => {
  if(getState().sequencer.selected === null) await addPattern(1)(dispatch)
}

// rows: a sample or a synth each. every row command goes by the row's id

// a sample dropped on the sequencer becomes a row at the bottom
export const addSampleRow = (path: string) => async (dispatch: Dispatch, getState: () => RootState) => {
  try {
    const { sequencer } = await invoke<AddedRow>('add_sample_row', { path })
    dispatch(setSequencer(sequencer))
    await addPatternIfNone(dispatch, getState)
  } catch(e) {
    sequencerError(`add ${path} to the sequencer`)(e)
  }
}

// its notes go from every pattern too, and a synth row's plugin with it
// (one undo brings them all back)
export const removeRow = (rowId: number) =>
  edit('remove the row', 'remove_row', { rowId })

// (one drag of the knob is one step to undo: see state/gesture.ts)
export const setRowVolume = (rowId: number, volume: number) =>
  editRow("set the row's volume", 'set_row_volume', { rowId, volume, gesture: currentGesture() })

// (a mute is always a step of its own)
export const setRowMuted = (rowId: number, muted: boolean) =>
  editRow('mute the row', 'set_row_muted', { rowId, muted, gesture: null })

// (one drag of the knob is one step to undo: see state/gesture.ts)
export const setRowPan = (rowId: number, pan: number) =>
  editRow("set the row's pan", 'set_row_pan', { rowId, pan, gesture: currentGesture() })

// a blank name shows the instrument's name again
export const renameRow = (rowId: number, name: string) =>
  editRow('rename the row', 'rename_row', { rowId, name })

// move a row to `index` in the list (counted without it). shown straight
// away, then as the engine has it
export const moveRow = (rowId: number, index: number) => (dispatch: Dispatch) => {
  dispatch(reduceRowMove({ rowId, index }))
  return invoke<SequencerState>('move_row', { rowId, index })
    .then(sequencer => { dispatch(setSequencer(sequencer)) })
    .catch(e => {
      sequencerError('move the row')(e)
      // and show the rows as the engine has them
      return loadSequencer()(dispatch)
    })
}

// play another sample on a sample row, keeping its notes, label, volume,
// pan and mute
export const replaceRowSample = (rowId: number, path: string) =>
  editRow(`replace the row's sample with ${path}`, 'replace_row_sample', { rowId, path })

// hear a sample row's sample, as it is
export const previewRow = (rowId: number) => () =>
  invoke('preview_row', { rowId }).catch(sequencerError('preview the row'))

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
  editPattern("change the pattern's length", 'set_pattern_bars', { patternId, bars })

// light or clear a row's step (see withStep)
export const setPatternStep = (patternId: number, rowId: number, step: number, on: boolean) =>
  (dispatch: Dispatch) => {
    dispatch(reduceStep({ patternId, rowId, step, on }))
    // every step painted in one stroke of the mouse is undone together
    const gesture = currentGesture()
    return editPattern('change the step', 'set_pattern_step', { patternId, rowId, step, on, gesture })(dispatch)
  }

// how often a pattern's notes are sent while they're being dragged
const SEND_NOTES_EVERY_MS = 30

// per pattern and row: the notes waiting to be sent, while a send was
// recent
interface QueuedNotes {
  patternId: number
  rowId: number
  notes: Note[]
  // the gesture they were changed in, taken when they were (it may be over
  // by the time they're sent)
  gesture: number | null
  dispatch: Dispatch
}
const queuedNotes = new Map<string, QueuedNotes | null>()

const queueKey = (patternId: number, rowId: number) => `${patternId}:${rowId}`

// counts the sends, so only the answer to the latest is kept: an earlier
// one arriving late would put back notes since dragged elsewhere. nor is an
// answer kept while newer notes are waiting to be sent
let noteSends = 0

const sendNotes = ({ patternId, rowId, notes, gesture, dispatch }: QueuedNotes): Promise<void> => {
  noteSends += 1
  const send = noteSends
  return invoke<PatternChange>('set_pattern_notes', { patternId, rowId, notes, gesture })
    .then(change => {
      if(send === noteSends && !queuedNotes.get(queueKey(patternId, rowId))) dispatch(reducePattern(change))
    })
    .catch(e => {
      sequencerError('change the notes')(e)
      // and show the notes as the engine has them
      return loadSequencer()(dispatch)
    })
}

// replace a row's notes in a pattern. put in order first, as the engine
// keeps them, so its answer doesn't move them about (anything kept by a
// note's place in the list has to follow them: see the Piano Roll's
// sortedWithPlaces). shown straight away, and sent at most every
// SEND_NOTES_EVERY_MS, the latest always last (flushNoteSends sends it now).
// every change made in one drag is one step to undo: see state/gesture.ts
export const setPatternNotes = (patternId: number, rowId: number, notes: Note[]) =>
  (dispatch: Dispatch): Promise<void> => {
    const sorted = sortNotes(notes)
    dispatch(reduceNotes({ patternId, rowId, notes: sorted }))
    const key = queueKey(patternId, rowId)
    const change = { patternId, rowId, notes: sorted, gesture: currentGesture(), dispatch }

    if(queuedNotes.has(key)) {
      queuedNotes.set(key, change)
      return Promise.resolve()
    }

    queuedNotes.set(key, null)
    const flush = () => {
      const next = queuedNotes.get(key)
      if(next) {
        queuedNotes.set(key, null)
        sendNotes(next)
        setTimeout(flush, SEND_NOTES_EVERY_MS)
      } else {
        queuedNotes.delete(key)
      }
    }
    setTimeout(flush, SEND_NOTES_EVERY_MS)

    return sendNotes(change)
  }

// send the notes still waiting, now: when a drag ends, and before an undo
// (a change has to have reached the engine to be undone)
export const flushNoteSends = (): Promise<unknown> =>
  Promise.all(Array.from(queuedNotes.entries()).map(([key, next]) => {
    if(!next) return undefined
    queuedNotes.set(key, null)
    return sendNotes(next)
  }))

// forget notes still waiting to be sent, and answers still to come, e.g.
// after an undo has replaced them
export const clearPendingNoteSends = () => {
  queuedNotes.clear()
  noteSends += 1
}

// the tabs that show the selected pattern, and loop it with Play while
// they're the focused pane's
const PATTERN_VIEWS = new Set([View.Sequencer, View.NoteSequencer, View.PianoRoll])

export const isPatternView = (view: View | undefined): boolean =>
  view !== undefined && PATTERN_VIEWS.has(view)

// which sequencer tab shows a pattern's notes, while there are two (one
// for the sample rows, one for the synth rows): the synths' when every note
// it holds is on a synth row, else the samples' (an empty pattern too)
export const sequencerTabFor = (pattern: Pattern, rows: Row[]): View => {
  const held = Object.keys(pattern.notes).map(Number)
  const onSynths = held.length > 0 && held.every(id => rows.some(row => row.id === id && isSynthRow(row)))
  return onSynths ? View.NoteSequencer : View.Sequencer
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
// playing a pattern of just that sample's notes (made in the pattern list, or
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

// open a pattern in the sequencer tab that shows its notes (double-clicking
// its clip, or picking it in the browser)
export const showPattern = (patternId: number) => (dispatch: Dispatch, getState: () => RootState) => {
  const { patterns, rows } = getState().sequencer
  const pattern = patterns.find(p => p.id === patternId)
  if(!pattern) return
  dispatch(selectPattern(patternId))
  dispatch(openTab(sequencerTabFor(pattern, rows)))
}

// open the Piano Roll on a synth row's notes in a pattern
export const showNotes = (patternId: number, rowId: number) => (dispatch: Dispatch) => {
  dispatch(selectPattern(patternId))
  dispatch(setPianoRollRow(rowId))
  dispatch(openTab(View.PianoRoll))
}

// loop a pattern on its own (its rows straight to the master), or stop it
// (or pause what's playing). `fromStep`: the first time through, from there
// (the Piano Roll's start marker)
export const togglePattern = (patternId: number, fromStep?: number) => async (dispatch: Dispatch) => {
  try {
    await invoke('toggle_pattern', fromStep ? { patternId, fromStep } : { patternId })
  } catch(e) {
    sequencerError('play the pattern')(e)
  }
  await syncRun(dispatch)
}

// while a pattern plays on its own, carry on from `step` (the Piano Roll's
// ruler was clicked)
export const seekPattern = (patternId: number, step: number) => async (dispatch: Dispatch) => {
  try {
    await invoke('seek_pattern', { patternId, step })
  } catch(e) {
    sequencerError('move the playhead')(e)
  }
  await syncRun(dispatch)
}

// the Play key: with a sequencer or the Piano Roll the focused pane's tab,
// loop the selected pattern (or pause what's playing); anywhere else, play
// the playlist as always
export const togglePlayHere = () => async (dispatch: Dispatch, getState: () => RootState) => {
  const state = getState()
  const view = selectFocusedView(state)
  const patternId = isPatternView(view) ? state.sequencer.selected : null

  if(patternId === null) {
    await togglePlay()(dispatch)
    dispatch(setRun(null))
    return
  }

  // the Piano Roll's from its start marker
  const fromStep = view === View.PianoRoll ? state.pianoRoll.startSteps[patternId] : undefined
  await togglePattern(patternId, fromStep)(dispatch)
}

// loop a pattern from its start, stopping whatever was playing
export const playPattern = (patternId: number) => async (dispatch: Dispatch) => {
  try {
    await invoke('play_pattern', { patternId })
  } catch(e) {
    sequencerError('play the pattern')(e)
  }
  await syncRun(dispatch)
}

// stop a pattern playing on its own (focus left the sequencers and the
// Piano Roll). the playlist's playhead stays where it was
export const stopPattern = () => async (dispatch: Dispatch) => {
  try {
    await invoke('pause_playlist', {})
  } catch(e) {
    sequencerError('stop the pattern')(e)
  }
  await syncRun(dispatch)
}

// synth rows. a synth row's plugin is in a slot, so its interface opens as
// an effect's does (see effectsSlice's openPlugin, which takes the slot)

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// an instrument that couldn't be added is said above the workspace, like an
// effect
const synthFail = (dispatch: Dispatch, what: string) => (e: unknown) => {
  dispatch(setProjectNotice({ kind: 'error', message: `${what}: ${message(e)}` }))
}

// an instrument dropped on the Note Sequencer: a synth row at the bottom.
// resolves to the row's id, or null when it couldn't be added (which is
// shown)
export const addSynthRow = (pluginId: string) =>
  async (dispatch: Dispatch, getState: () => RootState): Promise<number | null> => {
    try {
      const { rowId, sequencer } = await invoke<AddedRow>('add_synth_row', { pluginId })
      dispatch(setSequencer(sequencer))
      await addPatternIfNone(dispatch, getState)
      return rowId
    } catch(e) {
      synthFail(dispatch, 'Couldn’t add the instrument')(e)
      return null
    }
  }

// the playhead's beat, from the engine (null if it can't be asked)
const playheadBeat = async (): Promise<number | null> => {
  try {
    const t = await invoke<PlaylistTransport>('get_playlist_transport', {})
    return t.beatIntervalMs > 0 ? t.positionMs / t.beatIntervalMs : null
  } catch {
    return null
  }
}

// an instrument dropped on a track's mixer strip: a synth row, a blank
// pattern, and a clip of it on that track at the playhead, all one step to
// undo. the Piano Roll opens on it
export const addSynthOnTrack = (pluginId: string, trackNumber: number) =>
  async (dispatch: Dispatch): Promise<void> => {
    try {
      const dropBeat = await playheadBeat()
      const { rowId, patternId, project } = await invoke<{ rowId: number, patternId: number, project: ProjectView }>(
        'add_synth_on_track', { pluginId, trackNumber, dropBeat })
      dispatch(reduceProjectView({ project, selected: null }))
      showNotes(patternId, rowId)(dispatch)
    } catch(e) {
      synthFail(dispatch, 'Couldn’t add the instrument')(e)
    }
  }

export const selectRows = (state: RootState) => state.sequencer.rows
// each the same list until one of its rows changes (a synth row's knob
// leaves the sample rows' list as it was)
const sameItems = <T>(a: T[], b: T[]) => a.length === b.length && a.every((item, i) => item === b[i])
export const selectSampleRows = createSelector(
  [selectRows], rows => rows.filter(isSampleRow), { memoizeOptions: { resultEqualityCheck: sameItems } })
export const selectSynthRows = createSelector(
  [selectRows], rows => rows.filter(isSynthRow), { memoizeOptions: { resultEqualityCheck: sameItems } })
export const selectPatterns = (state: RootState) => state.sequencer.patterns
export const selectPatternFolders = (state: RootState) => state.sequencer.folders

export const selectSelectedPatternId = (state: RootState) => state.sequencer.selected
export const selectSelectedPattern = (state: RootState): Pattern | null =>
  state.sequencer.patterns.find(p => p.id === state.sequencer.selected) ?? null
// the pattern the playlist's Draw tool paints (see lastSelected)
export const selectLastSelectedPattern = (state: RootState) => state.sequencer.lastSelected
export const selectPianoRollRow = (state: RootState): Row<SynthInstrument> | null =>
  state.sequencer.rows.find((row): row is Row<SynthInstrument> =>
    row.id === state.sequencer.pianoRollRowId && isSynthRow(row)) ?? null
export const selectPatternRun = (state: RootState) => state.sequencer.run
