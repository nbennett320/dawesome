import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'
import { Note } from '../../types/sequencer'
import { GridScene, Viewport, drawGrid, gridSteps, notesBetween, rowTop } from './draw'

const note = (pitch: number, start: number, length = 1): Note => ({ pitch, start, length, velocity: 0.8 })

// a 2D context that draws nothing, and counts what it's asked to do
const counting = () => {
  const calls: Record<string, number> = {}
  const ctx = new Proxy({} as Record<string, unknown>, {
    get: (target, key: string) => (key in target ? target[key] : () => { calls[key] = (calls[key] ?? 0) + 1 }),
    set: (target, key: string, value) => {
      target[key] = value
      return true
    },
  }) as unknown as CanvasRenderingContext2D
  return { ctx, calls }
}

// 600 by 320 pixels, 24 a step: steps 100 to 125, pitches 61 down to 41
const view: Viewport = { width: 600, height: 320, scrollX: 100 * 24, scrollY: rowTop(61), stepWidth: 24 }
const scene = (more: Partial<GridScene>): GridScene => ({
  length: 128,
  limit: 2048,
  stepsPerBar: 16,
  notes: [],
  inOrder: true,
  ghosts: [],
  selected: new Set(),
  landing: null,
  lasso: null,
  cut: null,
  edgeBar: null,
  ...more,
})

// what of `notes` a drawing of `view` shows, worked out the long way
// (with a step's slack to the left)
const seen = (notes: Note[]) => notes.filter(n => n.pitch <= 61 && n.pitch >= 41 && n.start < 125 && n.start + n.length >= 99)

beforeAll(() => applyTheme(slateLightTheme))

describe('the notes in view', () => {
  // a note every step, and one long one from step 20 to 50
  const notes = [...Array.from({ length: 100 }, (_, i) => note(60, i)), note(62, 20, 30)].sort((a, b) => a.start - b.start)

  it('are found by binary search, from the first that could reach into the view', () => {
    const [first, end] = notesBetween(notes, 45, 60)
    // (the longest note is 30 steps: from those starting at 15)
    expect(notes[first].start).toBe(15)
    expect(notes[end].start).toBe(60)
    expect(notes.slice(first, end)).toContainEqual(note(62, 20, 30))
  })

  it('are none in a list with none, or past its last', () => {
    expect(notesBetween([], 0, 10)).toEqual([0, 0])
    expect(notesBetween(notes, 200, 210)).toEqual([notes.length, notes.length])
  })
})

describe('drawing the grid', () => {
  // four rows of ghosts, a note every step for 128 bars, most out of view.
  // one long note starts before the view and reaches into it
  const ghosts = [0, 1, 2, 3].map(g => Array.from({ length: 2048 }, (_, s) => note(44 + g * 4 + (s % 3), s)))
  ghosts[0] = [...ghosts[0], note(50, 90, 20)].sort((a, b) => a.start - b.start)

  it('draws only the ghosts in view, as one shape, the long one reaching in too', () => {
    const { ctx, calls } = counting()
    drawGrid(ctx, view, scene({ ghosts }))
    expect(seen(ghosts.flat())).toContainEqual(note(50, 90, 20))
    expect(calls.roundRect).toBe(seen(ghosts.flat()).length)
    expect(calls.roundRect).toBeLessThan(200)
    expect(calls.fill).toBe(1)
  })

  it('looks at the ghosts in view, not every one', () => {
    let looks = 0
    const watched = ghosts.map(row => row.map(n => new Proxy(n, {
      get: (target, key: keyof Note) => {
        if(key === 'pitch') looks += 1
        return target[key]
      },
    })))
    drawGrid(counting().ctx, view, scene({ ghosts: watched }))
    // (a note looked at has its pitch read twice, and once more to draw it.
    // looking at every one would be more than 16,000 reads)
    expect(looks).toBeLessThan(4 * seen(ghosts.flat()).length)
  })

  it('draws only the row’s own notes in view, in order or (mid-drag) not', () => {
    const notes = ghosts[0]
    const inOrder = counting()
    drawGrid(inOrder.ctx, view, scene({ notes }))
    expect(inOrder.calls.roundRect).toBe(2 * seen(notes).length)

    const dragged = counting()
    drawGrid(dragged.ctx, view, scene({ notes: [...notes].reverse(), inOrder: false }))
    expect(dragged.calls.roundRect).toBe(inOrder.calls.roundRect)
  })

  it('shades past the pattern’s end, and fills nothing past where time runs to', () => {
    const { ctx, calls } = counting()
    // the pattern ends in view, at step 112; time at 120
    drawGrid(ctx, view, scene({ length: 112, limit: 120 }))
    // the canvas, the lanes, the black keys in view, and past the end
    const blackKeys = [42, 44, 46, 49, 51, 54, 56, 58, 61].length
    expect(calls.fillRect).toBe(3 + blackKeys)
  })
})

describe('the width the grid scrolls through', () => {
  it('runs four bars past the pattern’s end, or past the furthest the view has seen, to the bar', () => {
    expect(gridSteps(16, 0, 16, 2048)).toBe(5 * 16)
    expect(gridSteps(16, 25, 16, 2048)).toBe(6 * 16)
    expect(gridSteps(16, 96, 16, 2048)).toBe(10 * 16)
    // 3/4: bars of 12 steps
    expect(gridSteps(24, 0, 12, 1536)).toBe(6 * 12)
  })

  it('never runs past where time runs to', () => {
    expect(gridSteps(16, 2040, 16, 2048)).toBe(2048)
    expect(gridSteps(2040, 0, 16, 2048)).toBe(2048)
  })
})
