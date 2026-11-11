export enum View {
  Playlist,
  // inspect a clip from the playlist and adjust how it plays
  SamplePreview,
  // every track's volume, pan, mute and solo, and the master
  Mixer,
  // the tape deck: a cassette and its play, pause, stop and wind keys
  Transport,
  // the Drum Sequencer: rows of samples, and drum patterns of 16th note
  // steps
  Sequencer,
  // which devices the engine plays and records through, and its buffer size
  AudioSettings,
  // File › Mixdown…: the song to a WAV or MP3 file
  Mixdown,
  // Dawesome › Preferences…: how the app looks
  Preferences,
  // one channel's effects, in the order the sound goes through them
  EffectsRack,
  // a pattern's notes on a grid of steps, against a piano keyboard
  PianoRoll,
  // one plugin's own interface (an effect's or a synth's). any number can
  // be open, one per slot
  Plugin,
  // the Note Sequencer: a row per synth, and note patterns of their notes
  NoteSequencer,
}

// the label a view's tab gets when it's opened
export const viewLabels: Record<View, string> = {
  [View.Playlist]: 'Playlist',
  [View.SamplePreview]: 'Sample',
  [View.Mixer]: 'Mixer',
  [View.Transport]: 'Transport',
  [View.Sequencer]: 'Drum Sequencer',
  [View.AudioSettings]: 'Audio',
  [View.Mixdown]: 'Mixdown',
  [View.Preferences]: 'Preferences',
  [View.EffectsRack]: 'Effects Rack',
  [View.PianoRoll]: 'Piano Roll',
  // (a plugin's tab is named after it, see pluginTabLabel)
  [View.Plugin]: 'Plugin',
  [View.NoteSequencer]: 'Note Sequencer',
}

// how a split pane lays out its children:
// 'vertical' is side by side (a vertical divider), 'horizontal' is stacked
export type SplitDirection = 'vertical' | 'horizontal'

export type PaneTab = {
  component: View
  // the effect or synth a View.Plugin tab shows
  slotId?: number
}

// what tells tabs apart: the view, and the slot for a plugin's tab
export type TabKey = string

export const tabKey = (view: View, slotId?: number): TabKey =>
  (slotId === undefined ? String(view) : `${view}:${slotId}`)

export const keyOf = (tab: PaneTab): TabKey => tabKey(tab.component, tab.slotId)

// a pane in the window layout (see state/paneTree.ts)
// - a leaf has `tabs` and `activeTab`, the selected tab's key
// - a split has `direction`, `left` and `right` (left is the top when stacked)
export type WindowNode = {
  id: string
  tabs: Array<PaneTab>
  activeTab?: TabKey
  direction?: SplitDirection
  left?: WindowNode
  right?: WindowNode
}

export type WindowPane = {
  id: string
  root: WindowNode
}

// the pages of the sidebar, each with a key on the rail down its left
export enum SidebarPage {
  // the samples folder, to hear samples and drag them into the playlist
  Samples = 'samples',
  // the Drum Sequencer's patterns, to drag into the playlist
  Patterns = 'patterns',
  // the Note Sequencer's patterns, likewise
  NotePatterns = 'note-patterns',
  // the project's recorded takes
  Recordings = 'recordings',
  // files the app wrote out itself: Deep copy's and Join's
  Clips = 'clips',
  // the effect plugins found, to drag onto a mixer channel
  Effects = 'effects',
  // the instrument plugins found, to drag into the Note Sequencer (or onto
  // a track's mixer strip)
  Instruments = 'instruments',
}
