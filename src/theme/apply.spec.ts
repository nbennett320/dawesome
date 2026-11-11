import { applyTheme, cssVar, currentTheme, onThemeChange } from './apply'
import { slateLightTheme } from './fixture'

const root = () => document.documentElement
const v = (name: string) => root().style.getPropertyValue(name)

describe('applyTheme', () => {
  it('sets core channels, fourtrack hex, scheme and data attributes', () => {
    applyTheme(slateLightTheme)
    expect(v('--c-surface')).toBe('255 255 255')
    expect(v('--c-ink-soft')).toBe('100 116 139')
    expect(v('--ft-cap-top')).toBe('#ffffff')
    expect(v('--ft-led-red-glow')).toBe('#ef444473')
    expect(root().style.colorScheme).toBe('light')
    expect(root().dataset.theme).toBe('slate-light')
    expect(root().dataset.appearance).toBe('light')
    expect(currentTheme()).toBe(slateLightTheme)
  })

  it('notifies listeners until unsubscribed', () => {
    const l = jest.fn()
    const off = onThemeChange(l)
    applyTheme(slateLightTheme)
    expect(l).toHaveBeenCalledTimes(1)
    off()
    applyTheme(slateLightTheme)
    expect(l).toHaveBeenCalledTimes(1)
  })
})

describe('cssVar', () => {
  it('builds rgb() strings', () => {
    expect(cssVar('inkSoft')).toBe('rgb(var(--c-ink-soft))')
    expect(cssVar('inkSoft', 0.5)).toBe('rgb(var(--c-ink-soft) / 0.5)')
  })
})
