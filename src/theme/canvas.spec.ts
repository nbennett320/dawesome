import { applyTheme } from './apply'
import { painter } from './canvas'
import { slateLightTheme } from './fixture'

const mockP = () => ({
  fill: jest.fn(),
  stroke: jest.fn(),
  background: jest.fn(),
})

beforeEach(() => applyTheme(slateLightTheme))

describe('painter', () => {
  const paint = painter('playlist')

  it('fills with the color', () => {
    const p = mockP()
    paint.fill(p, 'clipBorder')
    expect(p.fill).toHaveBeenCalledWith(203, 213, 225, 255)
  })

  it('multiplies the alpha', () => {
    const p = mockP()
    paint.fill(p, 'lassoFill', 0.15)
    expect(p.fill).toHaveBeenCalledWith(14, 165, 233, 255 * 0.15)
  })

  it('keeps a color own alpha', () => {
    const p = mockP()
    painter('waveform').fill(p, 'wash')
    expect(p.fill).toHaveBeenCalledWith(241, 245, 249, 153)
    expect(painter('waveform').css('wash')).toBe('rgba(241, 245, 249, 0.6)')
  })

  it('strokes, backgrounds and returns numbers', () => {
    const p = mockP()
    paint.stroke(p, 'clipBorder')
    paint.background(p, 'background')
    expect(p.stroke).toHaveBeenCalledWith(203, 213, 225, 255)
    expect(p.background).toHaveBeenCalledWith(255, 255, 255, 255)
    expect(paint.rgba('clip')).toHaveLength(4)
  })

  it('has a core painter', () => {
    expect(painter('colors').rgba('surface')).toEqual([255, 255, 255, 255])
  })

  it('rejects typos at the type level', () => {
    const p = mockP()
    // @ts-expect-error not a playlist key
    expect(() => paint.fill(p, 'nope')).toThrow()
    // @ts-expect-error not a core key
    expect(() => painter('colors').rgba('nope')).toThrow()
  })
})
