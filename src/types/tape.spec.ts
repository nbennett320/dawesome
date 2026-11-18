import {
  AGE, defaultTapeMachine, DRIVE_DB, formatGainDb, formatPercent, formatPitch, HISS, MACHINES, OUTPUT_DB, PITCH, WOBBLE,
} from './tape'

describe('tape', () => {
  it('starts off, on the 414, with fresh tape at the real deck’s wow, flutter and hiss', () => {
    expect(defaultTapeMachine).toEqual({
      enabled: false, machine: 0, age: 0, wobble: 100, hiss: 100, dbx: true, outputDb: 0, pitch: 0,
    })
    expect(MACHINES[defaultTapeMachine.machine]).toBe('Porta 414')
  })

  it('has the ranges agreed (T22), each default inside its range', () => {
    expect(AGE).toEqual({ min: 0, max: 100, default: 0 })
    expect(WOBBLE).toEqual({ min: 0, max: 200, default: 100 })
    expect(HISS).toEqual({ min: 0, max: 200, default: 100 })
    expect(OUTPUT_DB).toEqual({ min: -24, max: 12, default: 0 })
    expect(DRIVE_DB).toEqual({ min: -12, max: 18, default: 0 })
    expect(PITCH).toEqual({ min: -12, max: 12, default: 0 })
  })

  it('formats its values', () => {
    expect(formatPercent(99.6)).toBe('100%')
    expect(formatGainDb(3)).toBe('+3.0 dB')
    expect(formatGainDb(0)).toBe('0.0 dB')
    expect(formatGainDb(-0.01)).toBe('0.0 dB')
    expect(formatGainDb(-6.54)).toBe('-6.5 dB')
    expect(formatPitch(0)).toBe('0%')
    expect(formatPitch(-0.04)).toBe('0%')
    expect(formatPitch(3.46)).toBe('+3.5%')
    expect(formatPitch(-12)).toBe('-12.0%')
  })
})
