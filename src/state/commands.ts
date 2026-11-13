import { invoke } from '@tauri-apps/api/core'
import { closeFocusedTab, openTab } from './slices/windowSlice'
import { stopPlaylist, toggleLoop, toggleMetronome } from './slices/playlistSlice'
import { togglePlayHere } from './slices/sequencerSlice'
import { toggleCountIn } from './slices/recordingSlice'
import { quitIfSaved, runProjectCommand, saveAndQuit } from './slices/projectSlice'
import { menuViews } from '../events/window'
import type { AppDispatch } from './store'
import { View } from '../types/ui'

// what the info bar's command line (`:` then a name) can run, by name. one
// that resolves to a string is saying why it couldn't, for the bar to show
type Run = (dispatch: AppDispatch) => unknown

const tab = (view: View): Run => dispatch => dispatch(openTab(view))

const commands: Record<string, Run> = {
  // every tab the Window menu opens, by the same names
  ...Object.fromEntries(Object.entries(menuViews).map(([name, view]) => [name, tab(view)])),
  // shortcuts
  seq: tab(View.Sequencer),
  notes: tab(View.NoteSequencer),
  piano: tab(View.PianoRoll),
  fx: tab(View.EffectsRack),
  prefs: tab(View.Preferences),
  export: tab(View.Mixdown),

  play: dispatch => dispatch(togglePlayHere()),
  stop: dispatch => dispatch(stopPlaylist()),
  loop: dispatch => dispatch(toggleLoop()),
  metronome: dispatch => dispatch(toggleMetronome()),
  countin: dispatch => dispatch(toggleCountIn()),

  w: dispatch => dispatch(runProjectCommand('save')),
  // close tab
  q: dispatch => dispatch(closeFocusedTab()),
  'q!': dispatch => dispatch(closeFocusedTab()),
  wq: async dispatch => {
    await dispatch(runProjectCommand('save'))
    dispatch(closeFocusedTab())
  },

  // close window/quit
  qa: () => quitIfSaved()(),
  'qa!': () => invoke('quit'),
  wqa: dispatch => dispatch(saveAndQuit()),
}

// the command a typed line names without `:`
export const findCommand = (line: string): Run | undefined => {
  const name = line.trim().toLowerCase()
  return Object.prototype.hasOwnProperty.call(commands, name) ? commands[name] : undefined
}
