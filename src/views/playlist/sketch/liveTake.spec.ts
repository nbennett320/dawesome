import { liveTakeSpan } from './liveTake'

describe('the take being recorded', () => {
  it('reaches from where it started to the playhead', () => {
    expect(liveTakeSpan(4, 6.5)).toEqual({ start: 4, end: 6.5 })
  })

  it('shows nothing until the playhead has moved on from its start', () => {
    expect(liveTakeSpan(4, 4)).toBeNull()
  })

  it('starts again from the loop’s start once the loop comes round', () => {
    // before the engine's said so
    expect(liveTakeSpan(4, 1.5)).toEqual({ start: 0, end: 1.5 })
    expect(liveTakeSpan(4, 0)).toBeNull()
    // and after (it says the pass starts at the loop's start), past where
    // the first pass started
    expect(liveTakeSpan(0, 5)).toEqual({ start: 0, end: 5 })
  })

  it('starts again from a loop region’s start', () => {
    // from beat 10, in a region from beat 8: round to 8.5
    expect(liveTakeSpan(10, 8.5, 8)).toEqual({ start: 8, end: 8.5 })
    expect(liveTakeSpan(8, 9, 8)).toEqual({ start: 8, end: 9 })
  })
})
