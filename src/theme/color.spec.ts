import { channels, parseHex } from './color'

describe('parseHex', () => {
  it('parses #rgb', () => expect(parseHex('#0af')).toEqual([0, 170, 255, 255]))
  it('parses #rrggbb', () => expect(parseHex('#0ea5e9')).toEqual([14, 165, 233, 255]))
  it('parses #rrggbbaa', () => expect(parseHex('#f1f5f999')).toEqual([241, 245, 249, 153]))
  it('rejects junk', () => expect(() => parseHex('nope')).toThrow())
})

describe('channels', () => {
  it('gives space-separated r g b', () => {
    expect(channels('#64748b')).toBe('100 116 139')
    expect(channels('#ef444473')).toBe('239 68 68')
  })
})
