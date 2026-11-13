import { openTab } from './slices/windowSlice'
import { stopPlaylist, toggleLoop, toggleMetronome } from './slices/playlistSlice'
import { togglePlayHere } from './slices/sequencerSlice'
import { toggleCountIn } from './slices/recordingSlice'
import { runProjectCommand } from './slices/projectSlice'
import { menuViews } from '../events/window'
import type { AppDispatch } from './store'
import { View } from '../types/ui'

// what the info bar's command line (`:` then a name) can run, by name
type Run = (dispatch: AppDispatch) => unknown

const tab = (view: View): Run => dispatch => dispatch(openTab(view))

const commands: Record<string, Run> = {
  // every tab the Window menu opens, by the same names
  ...Object.fromEntries(Object.entries(menuViews).map(([name, view]) => [name, tab(view)])),
  // shorter names for those
  seq: tab(View.Sequencer),
  notes: tab(View.NoteSequencer),
  piano: tab(View.PianoRoll),
  fx: tab(View.EffectsRack),
  prefs: tab(View.Preferences),

  // the transport
  play: dispatch => dispatch(togglePlayHere()),
  stop: dispatch => dispatch(stopPlaylist()),
  loop: dispatch => dispatch(toggleLoop()),
  metronome: dispatch => dispatch(toggleMetronome()),
  countin: dispatch => dispatch(toggleCountIn()),

  // the project, as vim's :w
  w: dispatch => dispatch(runProjectCommand('save')),
}

// the command a typed line names (without its `:`), if any. case and
// surrounding space don't matter
export const findCommand = (line: string): Run | undefined => {
  const name = line.trim().toLowerCase()
  return Object.prototype.hasOwnProperty.call(commands, name) ? commands[name] : undefined
}
