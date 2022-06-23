export enum PlaylistTypes {
  PlaylistTrackItem = 'playlist-track-item',
  SidebarSampleItem = 'sidebar-sample-item',
}

export type PlaylistItem = {
  id: number,
  path: string,
  // offset: number,
  trackNumber: number,
  pixelOffset: number,
}

export type PlaylistItemWaveformData = {
  pathd: string,
  viewBox: string,
}
