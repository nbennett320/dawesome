// the mouse tools, the same in the playlist and the Piano Roll
import { View } from '../types/ui'

// what the mouse does to the items in a view:
// - select: move and trim them (shift+drag draws a box in every tool)
// - draw: make new ones on empty space; on an item, as Select
// - cut: split the one clicked
export type Tool = 'select' | 'draw' | 'cut'

export const TOOLS: ReadonlyArray<Tool> = ['select', 'draw', 'cut']

// each view's names for them, saying what they do to its items
export type ToolView = View.Playlist | View.PianoRoll

export const TOOL_LABELS: Record<ToolView, Record<Tool, string>> = {
  [View.Playlist]: {
    select: 'Select (S); select and move clips',
    draw: 'Draw (D); use your cursor to draw an array of clips',
    cut: 'Cut (C); split clips into pieces',
  },
  [View.PianoRoll]: {
    select: 'Select (S); select and move notes',
    draw: 'Draw (D); click to draw a note, drag right to set its length',
    cut: 'Cut (C); split notes into pieces',
  },
}

// S (select), D (draw) or C (cut) with no modifier. matched by the
// character typed, ignoring case, so Caps Lock makes no difference; ⌘S, ⌘D
// and ⌘C keep their meanings
export const toolKey = (
  ev: Pick<KeyboardEvent, 'key' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey'>,
): Tool | null => {
  if(ev.shiftKey || ev.metaKey || ev.ctrlKey || ev.altKey) return null
  const key = ev.key.toLowerCase()
  if(key === 's') return 'select'
  if(key === 'd') return 'draw'
  if(key === 'c') return 'cut'
  return null
}
