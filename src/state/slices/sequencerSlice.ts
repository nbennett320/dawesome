import { createSelector, createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { RootState } from '../store'
import {
  Note, Pattern, PatternKind, SequencerState, kindOf, withNotes, withStep,
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
  // the pattern of each kind that's shown, which Play loops: the Drum
  // Sequencer's drum pattern, and the Note Sequencer's and Piano Roll's
  // note pattern
  selected: Record<PatternKind, number | null>
  // the pattern most recently selected, made or shown in either sequencer,
  // of whichever kind: what the playlist's Draw tool paints. null when there
  // isn't one (it was deleted, or a project was opened since)
  lastSelected: number | null
  // the synth whose notes the Piano Roll edits
  pianoRollSynthId: number | null
  run: PatternRun | null
}

const initialState: SequencerSliceState = {
  rows: [],
  synths: [],
  patterns: [],
  folders: [],
  selected: { drum: null, note: null },
  lastSelected: null,
  pianoRollSynthId: null,
  run: null,
}

const KINDS: PatternKind[] = ['drum', 'note']

// keep a pattern of each kind selected while there are any: the one that
// was, or the first. and a synth for the Piano Roll while there are any
const fixSelection = (state: SequencerSliceState) => {
  KINDS.forEach(kind => {
    const ofKind = state.patterns.filter(p => kindOf(p) === kind)
    if(ofKind.some(p => p.id === state.selected[kind])) return
    state.selected[kind] = ofKind[0]?.id ?? null
  })
  // (not replaced by another: the Draw tool makes a pattern instead)
  if(!state.patterns.some(p => p.id === state.lastSelected)) state.lastSelected = null
  if(!state.synths.some(s => s.id === state.pianoRollSynthId)) {
    state.pianoRollSynthId = state.synths[0]?.id ?? null
  }
}

const take = (state: SequencerSliceState, sequencer: Partial<SequencerState> | null | undefined) => {
  state.rows = sequencer?.rows ?? []
  // (left out by older engines)
  state.synths = sequencer?.synths ?? []
  state.patterns = sequencer?.patterns ?? []
  state.folders = sequencer?.folders ?? []
}

export const sequencerSlice = createSlice({
  name: 'sequencerSlice',
  initialState,
  reducers: {
    // the sequencer as the engine has it, after any edit
    setSequencer: (state, action: PayloadAction<SequencerState>) => {
      take(state, action.payload)
      fixSelection(state)
    },
    // show a pattern in the sequencer of its kind
    selectPattern: (state, action: PayloadAction<number>) => {
      const pattern = state.patterns.find(p => p.id === action.payload)
      if(!pattern) return
      state.selected[kindOf(pattern)] = pattern.id
      state.lastSelected = pattern.id
    },
    // the synth whose notes the Piano Roll edits
    setPianoRollSynth: (state, action: PayloadAction<number>) => {
      if(state.synths.some(s => s.id === action.payload)) state.pianoRollSynthId = action.payload
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
    // and a synth's notes in a pattern, as they're drawn and dragged
    reduceNotes: (state, action: PayloadAction<{ patternId: number, synthId: number, notes: Note[] }>) => {
      const { patternId, synthId, notes } = action.payload
      const i = state.patterns.findIndex(p => p.id === patternId)
      if(i >= 0) state.patterns[i] = withNotes(state.patterns[i], synthId, notes)
    },
    setRun: (state, action: PayloadAction<PatternRun | null>) => {
      state.run = action.payload
    },
  },
  extraReducers: builder => {
    builder
      .addCase(reduceLoadedProject, (state, action) => {
        take(state, action.payload.sequencer)
        state.selected = { drum: null, note: null }
        state.lastSelected = null
        state.pianoRollSynthId = null
        state.run = null
        fixSelection(state)
      })
      // an undo or redo may have changed the rows, synths and patterns. the
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
        const pattern = state.patterns.find(p => p.id === action.payload.patternId)
        if(!pattern) return
        state.selected[kindOf(pattern)] = pattern.id
        state.lastSelected = pattern.id
      })
      // nothing plays once the engine has stopped
      .addCase(setPlaying, (state, action) => {
        if(!action.payload) state.run = null
      })
  },
})

export const { selectPattern, setPianoRollSynth, setSequencer } = sequencerSlice.actions
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

// a new empty pattern of a kind at the end, selected
export const addPattern = (bars: number, kind: PatternKind) => (dispatch: Dispatch) =>
  invoke<{ id: number, sequencer: SequencerState }>('add_pattern', { bars, kind })
    .then(({ id, sequencer }) => {
      dispatch(setSequencer(sequencer))
      dispatch(selectPattern(id))
    })
    .catch(sequencerError('add a pattern'))

// a sample dropped on the sequencer becomes a row. a project needs a
// pattern to put steps in, so the first row brings one along
export const addSequencerRow = (path: string) => async (dispatch: Dispatch, getState: () => RootState) => {
  await edit(`add ${path} to the sequencer`, 'add_sequencer_row', { path })(dispatch)
  if(getState().sequencer.selected.drum === null) await addPattern(1, 'drum')(dispatch)
}

export const removeSequencerRow = (rowId: number) =>
  edit('remove the row', 'remove_sequencer_row', { rowId })

// (one drag of the knob is one step to undo: see state/gesture.ts)
export const setSequencerRowVolume = (rowId: number, volume: number) =>
  edit("set the row's volume", 'set_sequencer_row_volume', { rowId, volume, gesture: currentGesture() })

export const setSequencerRowMuted = (rowId: number, muted: boolean) =>
  edit('mute the row', 'set_sequencer_row_muted', { rowId, muted })

// (one drag of the knob is one step to undo: see state/gesture.ts)
export const setSequencerRowPan = (rowId: number, pan: number) =>
  edit("set the row's pan", 'set_sequencer_row_pan', { rowId, pan, gesture: currentGesture() })

// a blank name shows the sample's name again
export const renameSequencerRow = (rowId: number, name: string) =>
  edit('rename the row', 'rename_sequencer_row', { rowId, name })

// move a row to `index` in the list
export const moveSequencerRow = (rowId: number, index: number) =>
  edit('move the row', 'move_sequencer_row', { rowId, index })

// play another sample on a row, keeping its steps, label, volume, pan and mute
export const replaceSequencerRowSample = (rowId: number, path: string) =>
  edit(`replace the row's sample with ${path}`, 'replace_sequencer_row_sample', { rowId, path })

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

// a new, empty folder of a kind in `parent` (null: the top). returns its id
export const addPatternFolder = (name: string, parent: number | null, kind: PatternKind) =>
  async (dispatch: Dispatch): Promise<number> => {
    const { id, sequencer } = await invoke<{ id: number, sequencer: SequencerState }>(
      'add_pattern_folder', { name, parent, kind })
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

// per pattern and synth: the notes waiting to be sent, while a send was
// recent
interface QueuedNotes {
  patternId: number
  synthId: number
  notes: Note[]
  // the gesture they were changed in, taken when they were (it may be over
  // by the time they're sent)
  gesture: number | null
  dispatch: Dispatch
}
const queuedNotes = new Map<string, QueuedNotes | null>()

const queueKey = (patternId: number, synthId: number) => `${patternId}:${synthId}`

// counts the sends, so only the answer to the latest is kept: an earlier
// one arriving late would put back notes since dragged elsewhere. nor is an
// answer kept while newer notes are waiting to be sent
let noteSends = 0

const sendNotes = ({ patternId, synthId, notes, gesture, dispatch }: QueuedNotes): Promise<void> => {
  noteSends += 1
  const send = noteSends
  return invoke<SequencerState>('set_pattern_notes', { patternId, synthId, notes, gesture })
    .then(sequencer => {
      if(send === noteSends && !queuedNotes.get(queueKey(patternId, synthId))) dispatch(setSequencer(sequencer))
    })
    .catch(e => {
      sequencerError('change the notes')(e)
      // and show the notes as the engine has them
      return loadSequencer()(dispatch)
    })
}

// replace a synth's notes in a pattern. shown straight away, and sent at
// most every SEND_NOTES_EVERY_MS, the latest always last (flushNoteSends
// sends it now). every change made in one drag is one step to undo: see
// state/gesture.ts
export const setPatternNotes = (patternId: number, synthId: number, notes: Note[]) =>
  (dispatch: Dispatch): Promise<void> => {
    dispatch(reduceNotes({ patternId, synthId, notes }))
    const key = queueKey(patternId, synthId)
    const change = { patternId, synthId, notes, gesture: currentGesture(), dispatch }

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

// the tabs that show a selected pattern, and loop it with Play while
// they're the focused pane's: which kind of pattern each shows
const PATTERN_VIEWS: Partial<Record<View, PatternKind>> = {
  [View.Sequencer]: 'drum',
  [View.NoteSequencer]: 'note',
  [View.PianoRoll]: 'note',
}

export const isPatternView = (view: View | undefined): boolean =>
  view !== undefined && PATTERN_VIEWS[view] !== undefined

// the sequencer tab for a kind of pattern
export const sequencerView = (kind: PatternKind): View =>
  (kind === 'drum' ? View.Sequencer : View.NoteSequencer)

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

// open a pattern in the sequencer of its kind (double-clicking its clip)
export const showPattern = (patternId: number) => (dispatch: Dispatch, getState: () => RootState) => {
  const pattern = getState().sequencer.patterns.find(p => p.id === patternId)
  if(!pattern) return
  dispatch(selectPattern(patternId))
  dispatch(openTab(sequencerView(kindOf(pattern))))
}

// open the Piano Roll on a synth's notes in a note pattern
export const showNotes = (patternId: number, synthId: number) => (dispatch: Dispatch) => {
  dispatch(selectPattern(patternId))
  dispatch(setPianoRollSynth(synthId))
  dispatch(openTab(View.PianoRoll))
}

// loop a pattern on its own (a note pattern's notes on its synths, straight
// to the master), or stop it (or pause what's playing). `fromStep`: the
// first time through, from there (the Piano Roll's start marker)
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
// loop its pattern (or pause what's playing); anywhere else, play the
// playlist as always
export const togglePlayHere = () => async (dispatch: Dispatch, getState: () => RootState) => {
  const state = getState()
  const view = selectFocusedView(state)
  const kind = view === undefined ? undefined : PATTERN_VIEWS[view]
  const patternId = kind ? state.sequencer.selected[kind] : null

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

// synths, the Note Sequencer's rows. a synth's id is also a slot id, so
// its interface opens as an effect's does (see effectsSlice's openPlugin)

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// a synth command that failed is shown above the workspace, like an
// effect's
const synthFail = (dispatch: Dispatch, what: string) => (e: unknown) => {
  dispatch(setProjectNotice({ kind: 'error', message: `${what}: ${message(e)}` }))
}

const synthEdit = (what: string, command: string, args: Record<string, unknown>) =>
  async (dispatch: Dispatch): Promise<void> => {
    try {
      dispatch(setSequencer(await invoke<SequencerState>(command, args)))
    } catch(e) {
      synthFail(dispatch, what)(e)
    }
  }

// an instrument dropped on the Note Sequencer: a synth at the end of the
// list. a project needs a note pattern to draw notes in, so the first synth
// brings one along. resolves to the synth's id, or null when it couldn't be
// added (which is shown)
export const addSynth = (pluginId: string) =>
  async (dispatch: Dispatch, getState: () => RootState): Promise<number | null> => {
    try {
      const { synthId, sequencer } = await invoke<{ synthId: number, sequencer: SequencerState }>(
        'add_synth', { pluginId })
      dispatch(setSequencer(sequencer))
      if(getState().sequencer.selected.note === null) await addPattern(1, 'note')(dispatch)
      return synthId
    } catch(e) {
      synthFail(dispatch, 'Couldn’t add the instrument')(e)
      return null
    }
  }

// its notes go from every pattern too (one undo brings both back)
export const removeSynth = (synthId: number) =>
  synthEdit('Couldn’t remove the instrument', 'remove_synth', { synthId })

// (one drag of the knob is one step to undo: see state/gesture.ts)
export const setSynthVolume = (synthId: number, volume: number) =>
  edit("set the synth's volume", 'set_synth_volume', { synthId, volume, gesture: currentGesture() })

export const setSynthMuted = (synthId: number, muted: boolean) =>
  synthEdit('Couldn’t mute the instrument', 'set_synth_muted', { synthId, muted })

// the playhead's beat, from the engine (null if it can't be asked)
const playheadBeat = async (): Promise<number | null> => {
  try {
    const t = await invoke<PlaylistTransport>('get_playlist_transport', {})
    return t.beatIntervalMs > 0 ? t.positionMs / t.beatIntervalMs : null
  } catch {
    return null
  }
}

// an instrument dropped on a track's mixer strip: a synth, a blank note
// pattern, and a clip of it on that track at the playhead, all one step to
// undo. the Piano Roll opens on it
export const addSynthOnTrack = (pluginId: string, trackNumber: number) =>
  async (dispatch: Dispatch): Promise<void> => {
    try {
      const dropBeat = await playheadBeat()
      const { synthId, patternId, project } = await invoke<{ synthId: number, patternId: number, project: ProjectView }>(
        'add_synth_on_track', { pluginId, trackNumber, dropBeat })
      dispatch(reduceProjectView({ project, selected: null }))
      showNotes(patternId, synthId)(dispatch)
    } catch(e) {
      synthFail(dispatch, 'Couldn’t add the instrument')(e)
    }
  }

export const selectSequencerRows = (state: RootState) => state.sequencer.rows
export const selectSynths = (state: RootState) => state.sequencer.synths
export const selectPatterns = (state: RootState) => state.sequencer.patterns
export const selectPatternFolders = (state: RootState) => state.sequencer.folders

// a kind's patterns and folders, each the same list until they change
const ofKind = <T extends { kind?: PatternKind }>(kind: PatternKind, pick: (state: RootState) => T[]) =>
  createSelector([pick], list => list.filter(item => (item.kind ?? 'drum') === kind))

const patternsByKind: Record<PatternKind, (state: RootState) => Pattern[]> = {
  drum: ofKind('drum', selectPatterns),
  note: ofKind('note', selectPatterns),
}
const foldersByKind: Record<PatternKind, (state: RootState) => SequencerState['folders']> = {
  drum: ofKind('drum', selectPatternFolders),
  note: ofKind('note', selectPatternFolders),
}

export const selectPatternsOf = (kind: PatternKind) => patternsByKind[kind]
export const selectFoldersOf = (kind: PatternKind) => foldersByKind[kind]

export const selectSelectedPatternId = (kind: PatternKind) => (state: RootState) =>
  state.sequencer.selected[kind]
export const selectSelectedPattern = (kind: PatternKind) => (state: RootState): Pattern | null =>
  state.sequencer.patterns.find(p => p.id === state.sequencer.selected[kind]) ?? null
// the pattern the playlist's Draw tool paints (see lastSelected)
export const selectLastSelectedPattern = (state: RootState) => state.sequencer.lastSelected
export const selectPianoRollSynth = (state: RootState) =>
  state.sequencer.synths.find(s => s.id === state.sequencer.pianoRollSynthId) ?? null
export const selectPatternRun = (state: RootState) => state.sequencer.run
