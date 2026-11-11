import { toolKey } from './tools'

describe('toolKey', () => {
  const press = (key: string, mods: Partial<Record<'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey', boolean>> = {}) =>
    toolKey({ key, shiftKey: false, metaKey: false, ctrlKey: false, altKey: false, ...mods })

  it('is select for S, draw for D and cut for C, whatever the case', () => {
    expect(press('s')).toBe('select')
    expect(press('S')).toBe('select')
    expect(press('d')).toBe('draw')
    expect(press('D')).toBe('draw')
    expect(press('c')).toBe('cut')
    expect(press('C')).toBe('cut')
  })

  it('is nothing with any modifier held', () => {
    ;(['shiftKey', 'metaKey', 'ctrlKey', 'altKey'] as const).forEach(mod => {
      expect(press('s', { [mod]: true })).toBeNull()
      expect(press('d', { [mod]: true })).toBeNull()
      expect(press('c', { [mod]: true })).toBeNull()
    })
  })

  it('is nothing for other keys', () => {
    expect(press('x')).toBeNull()
    expect(press('Escape')).toBeNull()
    expect(press(' ')).toBeNull()
  })
})
