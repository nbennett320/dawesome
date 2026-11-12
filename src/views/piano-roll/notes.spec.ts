import { Note, Pattern, clipNoteBars, stepsPerBar, synthNotes, withNotes } from '../../types/sequencer'
import {
  DEFAULT_LENGTH, NEW_VELOCITY, allIndices, carriedNotes, centrePitch, clampNote, copyNotes, cutNotes, deleteNotes, drawnLength,
  fitSelection, inSteps, isBlackKey, markerStep, moveNotes, newNote, noteBoxes, noteCut, pasteNotes, pasteStep, pitchName,
  pressKind, repeatNotes, rollSnapping, sameNotes, snapStep, snapStepDown, trimNotes,
} from './notes'
import {
  DEFAULT_STEP_WIDTH, MAX_STEP_WIDTH, MIN_STEP_WIDTH, ROW_HEIGHT, pitchAtY, rowAtY, rowTop, stepAtX, zoomGrid,
} from './draw'

const note = (pitch: number, start: number, length = 1): Note => ({ pitch, start, length, velocity: 0.8 })

const sixteenths = rollSnapping(true, 'SixteenthNote', false)
const beats = rollSnapping(true, 'QuarterNote', false)
const bypassed = rollSnapping(true, 'QuarterNote', true)
const off = rollSnapping(false, 'QuarterNote', false)

describe('snapping', () => {
  it('has a line every snap length, in steps, and no playhead', () => {
    expect(sixteenths).toEqual({ enabled: true, every: 1, playhead: null, bypass: false })
    expect(beats.every).toBe(4)
    expect(rollSnapping(true, 'WholeNote', false).every).toBe(16)
  })

  it('lands on the nearest line, or the line at or before for something new', () => {
    expect(snapStep(5.6, beats)).toBe(4)
    expect(snapStep(6.1, beats)).toBe(8)
    expect(snapStepDown(7.9, beats)).toBe(4)
  })

  it('keeps to whole steps with snap off, or Option held', () => {
    expect(inSteps(off).every).toBe(1)
    expect(inSteps(bypassed).every).toBe(1)
    expect(snapStep(5.6, off)).toBe(6)
    expect(snapStep(5.6, bypassed)).toBe(6)
    expect(snapStepDown(7.9, off)).toBe(7)
    expect(snapStepDown(7.9, bypassed)).toBe(7)
  })
})

describe('drawing a note', () => {
  it('starts on the snap line at or before the press, at the last length, with velocity 0.8', () => {
    expect(newNote(5.7, 60, DEFAULT_LENGTH, 16, sixteenths))
      .toEqual({ pitch: 60, start: 5, length: 4, velocity: NEW_VELOCITY })
    expect(DEFAULT_LENGTH).toBe(4)
    expect(newNote(5.7, 60, 2, 16, beats).start).toBe(4)
    expect(newNote(5.7, 60, 2, 16, bypassed).start).toBe(5)
  })

  it('is kept inside the pattern', () => {
    // drawn near the end, it's cut short where the pattern ends
    expect(newNote(14.2, 60, 4, 16, sixteenths)).toMatchObject({ start: 14, length: 2 })
    expect(newNote(15.2, 60, 4, 16, sixteenths)).toMatchObject({ start: 15, length: 1 })
    expect(newNote(-2, 200, 4, 16, sixteenths)).toMatchObject({ start: 0, pitch: 127 })
    expect(clampNote(note(-5, 3, 40), 16)).toMatchObject({ pitch: 0, start: 0, length: 16 })
  })

  it('takes the length dragged to: the nearest snap line, at least one snap length', () => {
    expect(drawnLength(4, 6.6, 16, sixteenths)).toBe(3)
    expect(drawnLength(4, 4.2, 16, sixteenths)).toBe(1)
    expect(drawnLength(4, 2, 16, sixteenths)).toBe(1)
    expect(drawnLength(4, 30, 16, sixteenths)).toBe(12)
    expect(drawnLength(4, 9.5, 16, beats)).toBe(4)
    expect(drawnLength(4, 5, 16, beats)).toBe(4)
    expect(drawnLength(4, 6.4, 16, off)).toBe(2)
    expect(drawnLength(12, 30, 16, beats)).toBe(4)
  })
})

describe('the mouse', () => {
  it('gives each note a box in steps and rows, its index for its id', () => {
    expect(noteBoxes([note(60, 2, 3), note(127, 0)])).toEqual([
      { id: 0, box: { left: 2, top: 67, width: 3, height: 1 } },
      { id: 1, box: { left: 0, top: 0, width: 1, height: 1 } },
    ])
  })

  it('knows what a press starts with each tool', () => {
    const at = { shift: false, onNote: false, onEdge: false, inPattern: true }
    // shift: a click or a box, whatever the tool and wherever
    expect(pressKind({ ...at, tool: 'select', shift: true, onNote: true, onEdge: true })).toBe('shift')
    expect(pressKind({ ...at, tool: 'cut', shift: true, onNote: true })).toBe('shift')
    expect(pressKind({ ...at, tool: 'draw', shift: true })).toBe('shift')

    expect(pressKind({ ...at, tool: 'select', onNote: true })).toBe('move')
    expect(pressKind({ ...at, tool: 'select', onNote: true, onEdge: true })).toBe('edge')
    expect(pressKind({ ...at, tool: 'select' })).toBe('empty')

    // Draw on a note is as Select
    expect(pressKind({ ...at, tool: 'draw', onNote: true })).toBe('move')
    expect(pressKind({ ...at, tool: 'draw', onNote: true, onEdge: true })).toBe('edge')
    expect(pressKind({ ...at, tool: 'draw' })).toBe('draw')
    expect(pressKind({ ...at, tool: 'draw', inPattern: false })).toBe('empty')

    // Cut leaves edges alone
    expect(pressKind({ ...at, tool: 'cut', onNote: true, onEdge: true })).toBe('cut')
    expect(pressKind({ ...at, tool: 'cut' })).toBe('empty')
  })
})

describe('the start marker', () => {
  it('goes on the nearest snap line inside the pattern', () => {
    expect(markerStep(5.4, 16, sixteenths)).toBe(5)
    expect(markerStep(5.6, 16, beats)).toBe(4)
    expect(markerStep(6.1, 16, beats)).toBe(8)
    expect(markerStep(5.6, 16, bypassed)).toBe(6)
    // never past the last line before the pattern's end, nor before it
    expect(markerStep(15.8, 16, sixteenths)).toBe(15)
    expect(markerStep(15.8, 16, beats)).toBe(12)
    expect(markerStep(-2, 16, beats)).toBe(0)
  })
})

describe('moving notes', () => {
  const notes = [note(60, 2, 2), note(64, 6, 2), note(70, 0, 1)]

  it('moves the chosen ones together by the grabbed one, to a step and a pitch', () => {
    expect(moveNotes(notes, [0, 1], 0, 5, 58, 16)).toEqual([note(58, 5, 2), note(62, 9, 2), note(70, 0, 1)])
    expect(moveNotes(notes, [0, 1], 1, 7, 64, 16)).toEqual([note(60, 3, 2), note(64, 7, 2), note(70, 0, 1)])
  })

  it('holds the move back at the pattern’s ends and the piano’s, keeping their shape', () => {
    expect(moveNotes(notes, [0, 1], 0, -10, 60, 16).slice(0, 2)).toEqual([note(60, 0, 2), note(64, 4, 2)])
    expect(moveNotes(notes, [0, 1], 0, 20, 60, 16).slice(0, 2)).toEqual([note(60, 10, 2), note(64, 14, 2)])
    expect(moveNotes(notes, [0, 1], 0, 2, 160, 16).slice(0, 2).map(n => n.pitch)).toEqual([123, 127])
    expect(moveNotes(notes, [0, 1], 0, 2, -40, 16).slice(0, 2).map(n => n.pitch)).toEqual([0, 4])
  })

  it('gives back the same list when nothing moves', () => {
    expect(moveNotes(notes, [0], 0, 2, 60, 16)).toBe(notes)
    expect(moveNotes(notes, [], 0, 3, 63, 16)).toBe(notes)
  })

  it('carries the chosen ones with the mouse, unsnapped, held back where they’d land', () => {
    const carried = carriedNotes(notes, [0, 1], 1.3, -0.4, 16)
    expect(carried.map(n => n.length)).toEqual([2, 2])
    ;[[3.3, 59.6], [7.3, 63.6]].forEach(([start, pitch], i) => {
      expect(carried[i].start).toBeCloseTo(start)
      expect(carried[i].pitch).toBeCloseTo(pitch)
    })
    // past the pattern's end and the top of the piano: stopped where
    // moveNotes stops them
    const far = carriedNotes(notes, [0, 1], 20, 100, 16)
    expect(far.map(n => [n.start, n.pitch])).toEqual([[10, 123], [14, 127]])
    expect(far.map(n => n.start)).toEqual(moveNotes(notes, [0, 1], 0, 20, 160, 16).slice(0, 2).map(n => n.start))
    expect(carriedNotes(notes, [], 1, 1, 16)).toEqual([])
  })
})

describe('trimming notes', () => {
  const notes = [note(60, 2, 2), note(64, 12, 2), note(67, 0, 1)]

  it('moves the right edge of each chosen note as far, each stopping at the pattern’s end', () => {
    expect(trimNotes(notes, [0, 1], 0, 'right', 7, 16)).toEqual([note(60, 2, 5), note(64, 12, 4), note(67, 0, 1)])
    // never shorter than a step
    expect(trimNotes(notes, [0], 0, 'right', -3, 16)[0]).toEqual(note(60, 2, 1))
  })

  it('moves the left edge too, ends staying put', () => {
    expect(trimNotes(notes, [0, 1], 0, 'left', 0, 16)).toEqual([note(60, 0, 4), note(64, 10, 4), note(67, 0, 1)])
    expect(trimNotes(notes, [0, 1], 0, 'left', 5, 16).slice(0, 2)).toEqual([note(60, 3, 1), note(64, 13, 1)])
    // the grabbed one is the one whose edge goes to the target
    expect(trimNotes(notes, [0, 1], 1, 'left', 11, 16).slice(0, 2)).toEqual([note(60, 1, 3), note(64, 11, 3)])
  })

  it('lets a note that already runs past the pattern’s end keep its end', () => {
    expect(trimNotes([note(60, 14, 4)], [0], 0, 'right', 17, 16)).toEqual([note(60, 14, 3)])
  })

  it('gives back the same list when nothing changes', () => {
    expect(trimNotes(notes, [0], 0, 'right', 4, 16)).toBe(notes)
  })
})

describe('cutting notes', () => {
  const notes = [note(60, 0, 4), { ...note(64, 0, 4), velocity: 0.5 }, note(67, 8, 2)]

  it('cuts the note clicked where the snap says, and the selection under the cut with it', () => {
    expect(noteCut(notes, 0, new Set([0, 1]), 2.3, sixteenths)).toEqual({ ids: [0, 1], step: 2 })
    // not selected: on its own
    expect(noteCut(notes, 0, new Set([1]), 2.3, sixteenths)).toEqual({ ids: [0], step: 2 })
    // a whole step with snap off
    expect(noteCut(notes, 0, new Set(), 2.6, off)).toEqual({ ids: [0], step: 3 })
  })

  it('cuts nowhere that leaves a half shorter than a step', () => {
    expect(noteCut(notes, 0, new Set(), 2.3, beats)).toBeNull()
    expect(noteCut(notes, 0, new Set(), 0.2, sixteenths)).toBeNull()
    expect(noteCut(notes, 7, new Set(), 2, sixteenths)).toBeNull()
  })

  it('makes each note two, the same pitch and velocity, the second halves after the rest', () => {
    const cut = cutNotes(notes, [0, 1], 3)
    expect(cut.notes).toEqual([
      note(60, 0, 3), { ...note(64, 0, 3), velocity: 0.5 }, note(67, 8, 2),
      note(60, 3, 1), { ...note(64, 3, 1), velocity: 0.5 },
    ])
    expect([...cut.halves]).toEqual([[0, 3], [1, 4]])
    expect(cutNotes(notes, [2], 3).notes).toBe(notes)
  })
})

describe('copying, pasting and repeating', () => {
  const notes = [note(60, 4, 2), note(64, 6, 2), note(67, 12, 2)]

  it('copies the selected notes in order', () => {
    expect(copyNotes(notes, new Set([1, 0]))).toEqual([notes[0], notes[1]])
    expect(copyNotes(notes, new Set([5]))).toEqual([])
  })

  it('pastes with the earliest at the step given, pitches kept, the copies after the notes', () => {
    expect(pasteNotes(notes, [notes[0], notes[1]], 0, 16)).toEqual({
      notes: [...notes, note(60, 0, 2), note(64, 2, 2)],
      added: [3, 4],
    })
  })

  it('leaves out a copy at or past the pattern’s end, and cuts short one crossing it', () => {
    expect(pasteNotes(notes, [notes[0], notes[1]], 13, 16).notes.slice(3)).toEqual([note(60, 13, 2), note(64, 15, 1)])
    expect(pasteNotes(notes, [notes[0], notes[1]], 15, 16)).toEqual({
      notes: [...notes, note(60, 15, 1)],
      added: [3],
    })
    expect(pasteNotes(notes, [notes[0]], 16, 16)).toEqual({ notes, added: [] })
    expect(pasteNotes(notes, [], 0, 16)).toEqual({ notes, added: [] })
  })

  it('repeats the selection where it ends, so again lays them end to end', () => {
    const once = repeatNotes(notes, new Set([0, 1]), 16)
    expect(once).toEqual({ notes: [...notes, note(60, 8, 2), note(64, 10, 2)], added: [3, 4] })
    expect(repeatNotes(once.notes, new Set(once.added), 16).notes.slice(5)).toEqual([note(60, 12, 2), note(64, 14, 2)])
    // only what starts before the pattern's end
    expect(repeatNotes(notes, new Set([0, 1, 2]), 16).notes.slice(3)).toEqual([note(60, 14, 2)])
  })

  it('pastes at the snap line at or before the mouse, or the first in view', () => {
    expect(pasteStep(5.7, 0, sixteenths)).toBe(5)
    expect(pasteStep(5.7, 0, beats)).toBe(4)
    expect(pasteStep(-1, 0, beats)).toBe(0)
    expect(pasteStep(null, 5.2, beats)).toBe(8)
    expect(pasteStep(null, 5.2, sixteenths)).toBe(6)
    expect(pasteStep(null, 8, beats)).toBe(8)
    expect(pasteStep(null, 0, beats)).toBe(0)
  })
})

describe('deleting and selecting', () => {
  const notes = [note(60, 0, 4), note(62, 4, 4), note(64, 8, 4), note(80, 0, 16)]

  it('deletes the chosen notes', () => {
    expect(deleteNotes(notes, [1, 3])).toEqual([notes[0], notes[2]])
    expect(deleteNotes(notes, new Set([0]))).toEqual(notes.slice(1))
  })

  it('selects all, and drops what’s no longer there', () => {
    expect([...allIndices(notes)]).toEqual([0, 1, 2, 3])
    const kept = new Set([0, 1])
    expect(fitSelection(kept, notes)).toBe(kept)
    expect([...fitSelection(new Set([0, 3, 5]), notes.slice(0, 2))]).toEqual([0])
  })

  it('tells lists of notes apart', () => {
    expect(sameNotes(notes, notes.map(n => ({ ...n })))).toBe(true)
    expect(sameNotes(notes, moveNotes(notes, [0], 0, 1, 60, 16))).toBe(false)
  })
})

describe('the grid', () => {
  it('names pitches with middle C as C4, and knows the black keys', () => {
    expect(pitchName(60)).toBe('C4')
    expect(pitchName(48)).toBe('C3')
    expect(pitchName(0)).toBe('C-1')
    expect(pitchName(61)).toBe('C#4')
    expect([60, 61, 62, 63, 64, 65, 66].map(isBlackKey)).toEqual([false, true, false, true, false, false, true])
  })

  it('opens on the middle of the notes, or middle C', () => {
    expect(centrePitch([])).toBe(60)
    expect(centrePitch([note(40, 0), note(50, 0)])).toBe(45)
  })

  it('turns pixels into steps, rows and pitches, and back, at any zoom', () => {
    expect(pitchAtY(rowTop(60))).toBe(60)
    expect(pitchAtY(rowTop(60) + ROW_HEIGHT - 1)).toBe(60)
    expect(pitchAtY(rowTop(127))).toBe(127)
    expect(rowAtY(rowTop(60) + ROW_HEIGHT / 2)).toBe(67.5)
    expect(stepAtX(DEFAULT_STEP_WIDTH * 2.5, DEFAULT_STEP_WIDTH)).toBe(2.5)
    expect(stepAtX(100, 50)).toBe(2)
  })

  it('zooms along time about the mouse, from 6 to 96 pixels a step', () => {
    expect(DEFAULT_STEP_WIDTH).toBe(24)
    expect(zoomGrid(24, 0, 2, 0)).toEqual({ stepWidth: 48, scrollLeft: 0 })
    // the step under the mouse (15) stays under it
    const zoomed = zoomGrid(24, 240, 2, 120)
    expect(zoomed).toEqual({ stepWidth: 48, scrollLeft: 600 })
    expect((zoomed.scrollLeft + 120) / zoomed.stepWidth).toBe(15)
    expect(zoomGrid(90, 0, 2, 0).stepWidth).toBe(MAX_STEP_WIDTH)
    const out = zoomGrid(8, 100, 0.1, 50)
    expect(out.stepWidth).toBe(MIN_STEP_WIDTH)
    expect((out.scrollLeft + 50) / out.stepWidth).toBeCloseTo((100 + 50) / 8)
    // never scrolled before the grid's start
    expect(zoomGrid(24, 0, 0.5, 300).scrollLeft).toBe(0)
  })

  it('has a bar of 16 steps in 4/4 and 12 in 3/4, as the Sequencer does', () => {
    expect(stepsPerBar(4)).toBe(16)
    expect(stepsPerBar(3)).toBe(12)
  })
})

describe('a pattern’s notes', () => {
  const pattern = (notes?: Record<string, Note[]>): Pattern =>
    ({ id: 0, name: null, kind: 'note', length: 16, steps: {}, notes })

  it('are none for a synth without any, or when an older engine leaves them out', () => {
    expect(synthNotes(pattern(), 3)).toEqual([])
    expect(synthNotes(pattern({ 4: [note(60, 0)] }), 3)).toBe(synthNotes(pattern(), 3))
    expect(synthNotes(pattern({ 4: [note(60, 0)] }), 4)).toEqual([note(60, 0)])
  })

  it('are replaced a synth at a time, a synth left without any left out', () => {
    const p = withNotes(pattern({ 4: [note(60, 0)] }), 5, [note(62, 1)])
    expect(p.notes).toEqual({ 4: [note(60, 0)], 5: [note(62, 1)] })
    expect(withNotes(p, 4, []).notes).toEqual({ 5: [note(62, 1)] })
  })

  it('show in a clip as bars, every synth’s: those starting in its part, cut at its end', () => {
    const p = pattern({ 1: [note(60, 0, 4), note(64, 6, 8)], 2: [note(72, 12, 2)] })
    expect(clipNoteBars(p, 0, 16)).toEqual({
      rows: 13,
      bars: [{ start: 0, end: 4, row: 12 }, { start: 6, end: 14, row: 8 }, { start: 12, end: 14, row: 0 }],
    })
    // trimmed to steps 4 to 10: the first note began before it, the second is cut short
    expect(clipNoteBars(p, 4, 10).bars).toEqual([{ start: 6, end: 10, row: 5 }])
    expect(clipNoteBars(pattern(), 0, 16)).toEqual({ rows: 0, bars: [] })
  })
})
