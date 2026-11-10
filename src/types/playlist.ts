export enum PlaylistTypes {
  PlaylistTrackItem = 'playlist-track-item',
  SidebarSampleItem = 'sidebar-sample-item',
}

export type PlaylistItemPixelOffset = {
  x: number,
  y: number,
  left: number,
  top: number,
  right: number,
  bottom: number,
  xOffset: number,
  yOffset: number,
}

export type PlaylistItem = {
  id: number,
  path: string,
  // where the clip starts, in beats from the start of the playlist
  // (after snapping), so it doesn't depend on the canvas size or zoom
  startBeat: number,
  trackNumber: number,
  pixelOffset: PlaylistItemPixelOffset,
  // set on a pattern clip: the pattern it plays (it has no sample, so its
  // path is empty)
  patternId?: number,
}

export type PlaylistItemWaveformData = {
  pathd: string,
  viewBox: string,
}

export type PlaylistWindow = {
  height: number,
  width: number,
  scrollPositionTop: number,
  scrollPositionLeft: number,
  scaleFactor: number,
}

// what the mouse does to clips in the playlist: move and trim them, or cut them
export type PlaylistTool = 'select' | 'cut'
