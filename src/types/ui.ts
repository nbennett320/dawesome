export enum View {
  Playlist,
  // inspect a clip from the playlist and adjust how it plays
  SamplePreview,
  // every track's volume, pan, mute and solo, and the master
  Mixer,
  // the tape deck: a cassette and its play, pause, stop and wind keys
  Transport,
  // the Pattern Sequencer: every row, sample or synth, and its steps (or a
  // picture of its notes) in the selected pattern
  PatternSequencer,
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
  // Preferences › Controls…: how the app is worked (plugins, command bar)
  Controls,
}

// the label a view's tab gets when it's opened
export const viewLabels: Record<View, string> = {
  [View.Playlist]: 'Playlist',
  [View.SamplePreview]: 'Sample',
  [View.Mixer]: 'Mixer',
  [View.Transport]: 'Transport',
  [View.PatternSequencer]: 'Pattern Sequencer',
  [View.AudioSettings]: 'Audio',
  [View.Mixdown]: 'Mixdown',
  [View.Preferences]: 'Preferences',
  [View.EffectsRack]: 'Effects Rack',
  [View.PianoRoll]: 'Piano Roll',
  // (a plugin's tab is named after it, see pluginTabLabel)
  [View.Plugin]: 'Plugin',
  [View.Controls]: 'Controls',
}

export const viewHints: Record<View, string> = {
  [View.Playlist]: 'The song: clips and patterns laid out along the timeline, a lane per track',
  [View.SamplePreview]: 'Inspect a clip from the playlist and adjust how it plays',
  [View.Mixer]: "Every track's volume, pan, mute and solo, and the master",
  [View.Transport]: 'The tape deck: a cassette and its play, pause, stop and wind keys',
  [View.PatternSequencer]: 'The Pattern Sequencer: a row per sample or synth, and its 16th-note steps or notes in the selected pattern',
  [View.AudioSettings]: 'Which devices the engine plays and records through, and its buffer size',
  [View.Mixdown]: 'Export the song to a WAV or MP3 file',
  [View.Preferences]: 'How the app looks: appearance, themes and step key style',
  [View.EffectsRack]: "One channel's effects, in the order the sound goes through them",
  [View.PianoRoll]: "A pattern's notes on a grid of steps, against a piano keyboard",
  [View.Plugin]: "A plugin's own interface",
  [View.Controls]: 'How the app is worked: where plugins open, and the command bar',
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
  // the project's patterns, to drag into the playlist
  Patterns = 'patterns',
  // the project's recorded takes
  Recordings = 'recordings',
  // files the app wrote out itself: Deep copy's and Join's
  Clips = 'clips',
  // the effect plugins found, to drag onto a mixer channel
  Effects = 'effects',
  // the instrument plugins found, to drag into the Pattern Sequencer (or
  // onto a track's mixer strip)
  Instruments = 'instruments',
}
