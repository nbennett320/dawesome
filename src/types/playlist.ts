export enum PlaylistTypes {
  PlaylistItem = 'playlist-item'
}

export type PlaylistItem = {
  id: number,
  path: string,
  offset: number,
}
