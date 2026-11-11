// the Piano Roll registers how it selects every note, so the Edit menu's
// Select All reaches it without the menu's code knowing about it. it does
// nothing unless the Piano Roll is the focused view (and the Playlist's
// own Select All does nothing unless the Playlist is)

let selectAll: (() => void) | null = null

export const setNoteSelectAllRunner = (run: (() => void) | null) => {
  selectAll = run
}

export const runNoteSelectAllFromMenu = (): void => {
  selectAll?.()
}
