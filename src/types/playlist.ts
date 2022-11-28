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
  // offset: number,
  trackNumber: number,
  pixelOffset: PlaylistItemPixelOffset,
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
