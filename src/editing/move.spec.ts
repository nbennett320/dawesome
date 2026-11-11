import { groupLanding } from './move'

describe('groupLanding', () => {
  const placed = [
    { id: 1, beat: 2, track: 0 },
    { id: 2, beat: 2.25, track: 1 },
    { id: 3, beat: 6, track: 1 },
  ]

  it('moves every clip by as much as the grabbed one', () => {
    expect(groupLanding(placed, 2, 4.25, 2, 16, 4)).toEqual([
      { id: 1, beat: 4, track: 1 },
      { id: 2, beat: 4.25, track: 2 },
      { id: 3, beat: 8, track: 2 },
    ])
  })

  it('stops the group at the start and end of the playlist', () => {
    expect(groupLanding(placed, 3, 3, 1, 16, 4).map(c => c.beat)).toEqual([0, 0.25, 4])
    expect(groupLanding(placed, 1, 15, 0, 16, 4).map(c => c.beat)).toEqual([12, 12.25, 16])
  })

  it('stops the group at the first track and the new-track area', () => {
    expect(groupLanding(placed, 2, 2.25, 0, 16, 4).map(c => c.track)).toEqual([0, 1, 1])
    expect(groupLanding(placed, 1, 2, 9, 16, 4).map(c => c.track)).toEqual([3, 4, 4])
  })

  it('lands nothing without the grabbed clip', () => {
    expect(groupLanding(placed, 9, 0, 0, 16, 4)).toEqual([])
  })
})

describe('groupLanding, for items with a length', () => {
  // notes: steps and pitches
  const notes = [
    { id: 0, beat: 0, track: 60, length: 4 },
    { id: 1, beat: 4, track: 64, length: 8 },
  ]

  it('keeps every end inside too', () => {
    // 16 steps: the second note ends on step 12, so there are 4 to spare
    expect(groupLanding(notes, 0, 10, 60, 16, 127).map(n => n.beat)).toEqual([4, 8])
  })

  it('keeps what else an item carries', () => {
    expect(groupLanding(notes, 0, 1, 62, 16, 127)).toEqual([
      { id: 0, beat: 1, track: 62, length: 4 },
      { id: 1, beat: 5, track: 66, length: 8 },
    ])
  })

  it('holds a group that already reaches past the end where it is, rather than pulling it back', () => {
    const past = [{ id: 0, beat: 14, track: 60, length: 4 }]
    expect(groupLanding(past, 0, 15, 60, 16, 127).map(n => n.beat)).toEqual([14])
    expect(groupLanding(past, 0, 10, 60, 16, 127).map(n => n.beat)).toEqual([10])
  })
})
