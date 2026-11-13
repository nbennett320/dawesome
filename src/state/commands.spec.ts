import { describe, expect, it, vi } from 'vitest'
import { findCommand } from './commands'
import { View } from '../types/ui'

describe('findCommand', () => {
  it('opens a tab by its Window menu name', () => {
    const dispatch = vi.fn()
    findCommand('mixer')!(dispatch)
    expect(dispatch).toHaveBeenCalledWith(expect.objectContaining({ payload: View.Mixer }))
  })

  it('takes the short names', () => {
    const dispatch = vi.fn()
    findCommand('seq')!(dispatch)
    expect(dispatch).toHaveBeenCalledWith(expect.objectContaining({ payload: View.PatternSequencer }))
  })

  it('opens the Pattern Sequencer by its menu name and its old one', () => {
    const dispatch = vi.fn()
    findCommand('sequencer')!(dispatch)
    findCommand('notes')!(dispatch)
    expect(dispatch.mock.calls.map(([action]) => action.payload)).toEqual([View.PatternSequencer, View.PatternSequencer])
    expect(findCommand('note-sequencer')).toBeUndefined()
  })

  it('ignores case and surrounding space', () => {
    expect(findCommand('  Mixer ')).toBe(findCommand('mixer'))
  })

  it('finds nothing for an unknown name', () => {
    expect(findCommand('nope')).toBeUndefined()
    expect(findCommand('')).toBeUndefined()
    // not something every object has
    expect(findCommand('toString')).toBeUndefined()
  })
})
