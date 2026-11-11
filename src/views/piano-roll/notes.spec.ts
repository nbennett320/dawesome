import { Note, Pattern, clipNoteBars, notesOf, stepsPerBar } from '../../types/sequencer'
import {
  DEFAULT_LENGTH, NEW_VELOCITY, allIndices, centrePitch, clampNote, deleteNotes, drawnLength, fitSelection,
  hitNote, isBlackKey, moveNotes, newNote, notesInBox, pitchName, resizeNotes, sameNotes, toggleSelected,
} from './notes'
import { pitchAtY, rowTop, stepAtX, ROW_HEIGHT, STEP_WIDTH } from './draw'

const note = (pitch: number, start: number, length = 1): Note => ({ pitch, start, length, velocity: 0.8 })

describe('drawing a note', () => {
  it('starts on the whole step pressed, at the last length, with velocity 0.8', () => {
    expect(newNote(5.7, 60, DEFAULT_LENGTH, 16)).toEqual({ pitch: 60, start: 5, length: 4, velocity: NEW_VELOCITY })
    expect(DEFAULT_LENGTH).toBe(4)
  })

  it('is kept inside the pattern', () => {
    // drawn near the end, it's cut short where the pattern ends
    expect(newNote(14.2, 60, 4, 16)).toMatchObject({ start: 14, length: 2 })
    expect(newNote(15.2, 60, 4, 16)).toMatchObject({ start: 15, length: 1 })
    expect(newNote(-2, 200, 4, 16)).toMatchObject({ start: 0, pitch: 127 })
    expect(clampNote(note(-5, 3, 40), 16)).toMatchObject({ pitch: 0, start: 0, length: 16 })
  })

  it('takes the length dragged to: up to the step the mouse is in, at least one', () => {
    expect(drawnLength(4, 4.2, 16)).toBe(1)
    expect(drawnLength(4, 7.5, 16)).toBe(4)
    expect(drawnLength(4, 2, 16)).toBe(1)
    expect(drawnLength(4, 30, 16)).toBe(12)
  })
})

describe('hitting a note', () => {
  const notes = [note(60, 0, 4), note(60, 2, 4), note(62, 8, 1)]
  const edge = 6 / STEP_WIDTH

  it('finds the one on top (drawn last), by its body or its right edge', () => {
    expect(hitNote(notes, 3, 60, edge)).toEqual({ index: 1, part: 'body' })
    expect(hitNote(notes, 1, 60, edge)).toEqual({ index: 0, part: 'body' })
    expect(hitNote(notes, 5.9, 60, edge)).toEqual({ index: 1, part: 'end' })
    expect(hitNote(notes, 6, 60, edge)).toBeNull()
    expect(hitNote(notes, 3, 61, edge)).toBeNull()
  })

  it('leaves a short note a body to grab', () => {
    expect(hitNote(notes, 8.5, 62, edge)).toEqual({ index: 2, part: 'body' })
    expect(hitNote(notes, 8.9, 62, edge)).toEqual({ index: 2, part: 'end' })
  })
})

describe('moving notes', () => {
  const notes = [note(60, 2, 2), note(64, 6, 2), note(70, 0, 1)]

  it('moves the chosen ones together by whole steps and semitones', () => {
    expect(moveNotes(notes, [0, 1], 3, -2, 16)).toEqual([note(58, 5, 2), note(62, 9, 2), note(70, 0, 1)])
    expect(moveNotes(notes, [0], 1.4, 0.4, 16)[0]).toEqual(note(60, 3, 2))
  })

  it('holds the move back at the pattern’s ends and the piano’s, keeping their shape', () => {
    expect(moveNotes(notes, [0, 1], -10, 0, 16).slice(0, 2)).toEqual([note(60, 0, 2), note(64, 4, 2)])
    expect(moveNotes(notes, [0, 1], 20, 0, 16).slice(0, 2)).toEqual([note(60, 10, 2), note(64, 14, 2)])
    expect(moveNotes(notes, [0, 1], 0, 100, 16).slice(0, 2).map(n => n.pitch)).toEqual([123, 127])
    expect(moveNotes(notes, [0, 1], 0, -100, 16).slice(0, 2).map(n => n.pitch)).toEqual([0, 4])
  })

  it('gives back the same list when nothing moves', () => {
    expect(moveNotes(notes, [0], 0, 0, 16)).toBe(notes)
    expect(moveNotes(notes, [], 3, 3, 16)).toBe(notes)
  })
})

describe('resizing notes', () => {
  const notes = [note(60, 2, 2), note(64, 12, 2)]

  it('changes each chosen note’s length, at least a step, inside the pattern', () => {
    expect(resizeNotes(notes, [0, 1], 3, 16)).toEqual([note(60, 2, 5), note(64, 12, 4)])
    expect(resizeNotes(notes, [0], -5, 16)[0]).toEqual(note(60, 2, 1))
    expect(resizeNotes(notes, [1], 0.4, 16)).toBe(notes)
  })
})

describe('deleting and selecting', () => {
  const notes = [note(60, 0, 4), note(62, 4, 4), note(64, 8, 4), note(80, 0, 16)]

  it('deletes the chosen notes', () => {
    expect(deleteNotes(notes, [1, 3])).toEqual([notes[0], notes[2]])
  })

  it('shift-clicks in and out of the selection', () => {
    expect(toggleSelected([2], 0)).toEqual([0, 2])
    expect(toggleSelected([0, 2], 2)).toEqual([0])
  })

  it('selects what a rubber band touches, whichever way it was dragged', () => {
    expect(notesInBox(notes, { step: 5, pitch: 60 }, { step: 3.5, pitch: 65 })).toEqual([0, 1])
    expect(notesInBox(notes, { step: 9, pitch: 90 }, { step: 0.5, pitch: 63 })).toEqual([2, 3])
  })

  it('selects all, and drops what’s no longer there', () => {
    expect(allIndices(notes)).toEqual([0, 1, 2, 3])
    const kept = [0, 1]
    expect(fitSelection(kept, notes)).toBe(kept)
    expect(fitSelection([0, 3, 5], notes.slice(0, 2))).toEqual([0])
  })

  it('tells lists of notes apart', () => {
    expect(sameNotes(notes, notes.map(n => ({ ...n })))).toBe(true)
    expect(sameNotes(notes, moveNotes(notes, [0], 1, 0, 16))).toBe(false)
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

  it('turns pixels into steps and pitches, and back', () => {
    expect(pitchAtY(rowTop(60))).toBe(60)
    expect(pitchAtY(rowTop(60) + ROW_HEIGHT - 1)).toBe(60)
    expect(pitchAtY(rowTop(127))).toBe(127)
    expect(stepAtX(STEP_WIDTH * 2.5)).toBe(2.5)
  })

  it('has a bar of 16 steps in 4/4 and 12 in 3/4, as the Sequencer does', () => {
    expect(stepsPerBar(4)).toBe(16)
    expect(stepsPerBar(3)).toBe(12)
  })
})

describe('a pattern’s notes', () => {
  const pattern = (notes?: Note[]): Pattern => ({ id: 0, name: null, length: 16, steps: {}, notes })

  it('are none when an older engine leaves them out', () => {
    expect(notesOf(pattern())).toEqual([])
    expect(notesOf(pattern())).toBe(notesOf(pattern()))
  })

  it('show in a clip as bars: those starting in its part, cut at its end', () => {
    const p = pattern([note(60, 0, 4), note(64, 6, 8), note(72, 12, 2)])
    expect(clipNoteBars(p, 0, 16)).toEqual({
      rows: 13,
      bars: [{ start: 0, end: 4, row: 12 }, { start: 6, end: 14, row: 8 }, { start: 12, end: 14, row: 0 }],
    })
    // trimmed to steps 4 to 10: the first note began before it, the second is cut short
    expect(clipNoteBars(p, 4, 10).bars).toEqual([{ start: 6, end: 10, row: 5 }])
    expect(clipNoteBars(pattern(), 0, 16)).toEqual({ rows: 0, bars: [] })
  })
})
