export enum View {
  Playlist,
  // inspect a clip from the playlist and adjust how it plays
  SamplePreview,
  // every track's volume, pan, mute and solo, and the master
  Mixer,
  // the tape deck: a cassette and its play, pause, stop and wind keys
  Transport,
  // the drum machine: rows of samples, and patterns of 16th note steps
  Sequencer,
  // which devices the engine plays and records through, and its buffer size
  AudioSettings,
}

// the label a view's tab gets when it's opened
export const viewLabels: Record<View, string> = {
  [View.Playlist]: 'Playlist',
  [View.SamplePreview]: 'Sample',
  [View.Mixer]: 'Mixer',
  [View.Transport]: 'Transport',
  [View.Sequencer]: 'Sequencer',
  [View.AudioSettings]: 'Audio',
}

// how a split pane lays out its children, named like react-split-pane:
// 'vertical' is side by side (a vertical divider), 'horizontal' is stacked
export type SplitDirection = 'vertical' | 'horizontal'

export type PaneTab = {
  label: string
  component: View
}

// a pane in the window layout (see state/paneTree.ts)
// - a leaf has `tabs` and `activeView`, the selected tab
// - a split has `direction`, `left` and `right` (left is the top when stacked)
export type WindowNode = {
  id: string
  tabs: Array<PaneTab>
  activeView?: View
  direction?: SplitDirection
  left?: WindowNode
  right?: WindowNode
}

export type WindowPane = {
  id: string
  root: WindowNode
}
