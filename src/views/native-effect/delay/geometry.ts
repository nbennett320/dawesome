// where things are on the Delay's echo view, in the area's own pixels: time
// across (from the sound at 0 to `span` seconds), and each echo a bar from
// the middle, up for the left side and down for the right, as tall as it's
// loud (FLOOR_DB at the middle, 0 dB at the top or bottom)
import { FLOOR_DB } from './echoes'

export interface Area {
  width: number
  height: number
}

// room on the left for the side and dB labels, and on the right
export const LEFT = 28
export const RIGHT = 12
// room above and below the tallest bars, for the caption and time labels
export const PAD = 18
// how near a press must be to an echo to take it
export const HIT = 10

export const timeToX = (seconds: number, span: number, area: Area): number =>
  LEFT + (seconds / Math.max(1e-6, span)) * Math.max(1, area.width - LEFT - RIGHT)

export const xToTime = (x: number, span: number, area: Area): number =>
  Math.max(0, ((x - LEFT) / Math.max(1, area.width - LEFT - RIGHT)) * span)

export const middle = (area: Area): number => area.height / 2

// how tall an echo's bar is
export const barHeight = (db: number, area: Area): number =>
  Math.max(0, 1 - db / FLOOR_DB) * Math.max(1, middle(area) - PAD)
