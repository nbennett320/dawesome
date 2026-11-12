import { describe, expect, it } from 'vitest'
import { clamp, clampLowWins, cx, fileName, plural } from './helpers'

describe('clamp', () => {
  it('keeps a number within the range', () => {
    expect(clamp(5, 0, 10)).toBe(5)
    expect(clamp(-1, 0, 10)).toBe(0)
    expect(clamp(11, 0, 10)).toBe(10)
  })
})

describe('clampLowWins', () => {
  it('works like clamp for a range the right way round', () => {
    expect(clampLowWins(5, 0, 10)).toBe(5)
    expect(clampLowWins(-1, 0, 10)).toBe(0)
    expect(clampLowWins(11, 0, 10)).toBe(10)
  })

  it('gives the low end when the range is backwards', () => {
    expect(clampLowWins(5, 8, 2)).toBe(8)
    expect(clampLowWins(1, 8, 2)).toBe(8)
    expect(clampLowWins(20, 8, 2)).toBe(8)
  })
})

describe('cx', () => {
  it('joins the names that are set', () => {
    expect(cx('a', false, null, undefined, 'b')).toBe('a b')
    expect(cx()).toBe('')
  })
})

describe('fileName', () => {
  it('is the last part of a path, with either kind of slash', () => {
    expect(fileName('/music/kick.wav')).toBe('kick.wav')
    expect(fileName('C:\\music\\kick.wav')).toBe('kick.wav')
    expect(fileName('kick.wav')).toBe('kick.wav')
  })

  it('ignores a trailing slash', () => {
    expect(fileName('/plugins/Synth.vst3/')).toBe('Synth.vst3')
  })
})

describe('plural', () => {
  it('counts one of something and many of it', () => {
    expect(plural(1, 'clip')).toBe('1 clip')
    expect(plural(0, 'clip')).toBe('0 clips')
    expect(plural(3, 'clip')).toBe('3 clips')
  })
})
