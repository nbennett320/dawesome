import { ClipWindow } from '../../types/clip'

// a horizontal span of the waveform canvas, pixels
export interface Region {
  left: number
  width: number
}

// the part of a canvas `width` pixels wide, drawn across the whole sample,
// that the clip plays. all of it when the clip has no window
export const playedRegion = (
  window: ClipWindow | null | undefined,
  sampleDuration: number,
  width: number,
): Region => {
  if(!window || sampleDuration <= 0) return { left: 0, width }

  // a window past the sample's ends is cut to them
  const start = Math.max(0, Math.min(sampleDuration, window.start))
  const end = Math.max(start, Math.min(sampleDuration, window.end ?? sampleDuration))

  return {
    left: start / sampleDuration * width,
    width: (end - start) / sampleDuration * width,
  }
}

// x of a time (seconds) within the played part, `played` seconds long
export const xAtTime = (region: Region, played: number, time: number): number =>
  played > 0 ? region.left + time / played * region.width : region.left

// the time within the played part at x, held to the region's ends
export const timeAtX = (region: Region, played: number, x: number): number => {
  if(region.width <= 0) return 0
  const fraction = Math.max(0, Math.min(1, (x - region.left) / region.width))
  return fraction * played
}
