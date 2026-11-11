// (rendering it isn't tested: with esModuleInterop off, React's default
// import was undefined under jest)
import { wheelMove } from './RollingCounter'

describe('wheelMove', () => {
  it('rolls forward one digit', () => {
    expect(wheelMove(3, 4, true)).toEqual({ kind: 'roll', from: 3, to: 4 })
  })

  it('rolls 9 on round to 0, the same way', () => {
    expect(wheelMove(9, 0, true)).toEqual({ kind: 'roll', from: 9, to: 10 })
  })

  it('rolls back one digit', () => {
    expect(wheelMove(4, 3, true)).toEqual({ kind: 'roll', from: 4, to: 3 })
  })

  it('rolls 0 back round to 9', () => {
    expect(wheelMove(0, 9, true)).toEqual({ kind: 'roll', from: 10, to: 9 })
  })

  it('jumps further than one digit', () => {
    expect(wheelMove(2, 7, true)).toEqual({ kind: 'jump', to: 7 })
  })

  it('jumps when changes come too fast to roll', () => {
    expect(wheelMove(3, 4, false)).toEqual({ kind: 'jump', to: 4 })
  })

  it('jumps to its first digit', () => {
    expect(wheelMove(null, 5, true)).toEqual({ kind: 'jump', to: 5 })
  })

  it('stays put when the digit is the same', () => {
    expect(wheelMove(5, 5, true)).toEqual({ kind: 'jump', to: 5 })
  })
})
