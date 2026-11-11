import { useEffect, useState } from 'react'
import { currentTheme, onThemeChange } from './apply'
import { parseHex, RGBA } from './color'
import type { CoreKey, ResolvedTheme, SectionKey, SectionName } from './keys'

/** the slice of a p5 instance the painter uses */
export interface P5Like {
  fill(r: number, g: number, b: number, a?: number): unknown
  stroke(r: number, g: number, b: number, a?: number): unknown
  background(r: number, g: number, b: number, a?: number): unknown
}

export interface Painter<K extends string> {
  fill(p: P5Like, key: K, alpha?: number): void
  stroke(p: P5Like, key: K, alpha?: number): void
  background(p: P5Like, key: K, alpha?: number): void
  /** [r, g, b, a], each 0-255 */
  rgba(key: K, alpha?: number): RGBA
  /** 'rgba(r, g, b, a)' for 2D canvas and inline styles */
  css(key: K, alpha?: number): string
}

const cache = new WeakMap<ResolvedTheme, Map<string, RGBA>>()

const lookup = (name: string): RGBA => {
  const theme = currentTheme()
  if(!theme) throw new Error(`no theme applied (reading ${name})`)
  let map = cache.get(theme)
  if(!map) cache.set(theme, (map = new Map()))
  let c = map.get(name)
  if(!c) {
    const hex = (theme.colors as Record<string, string>)[name]
    if(hex === undefined) throw new Error(`unknown theme key: ${name}`)
    map.set(name, (c = parseHex(hex)))
  }
  return c
}

const make = <K extends string>(prefix: string): Painter<K> => {
  // `alpha` multiplies the color's own alpha
  const rgba = (key: K, alpha = 1): RGBA => {
    const [r, g, b, a] = lookup(prefix + key)
    return [r, g, b, a * alpha]
  }
  return {
    rgba,
    css: (key, alpha) => {
      const [r, g, b, a] = rgba(key, alpha)
      return `rgba(${r}, ${g}, ${b}, ${+(a / 255).toFixed(3)})`
    },
    fill: (p, key, alpha) => {
      p.fill(...rgba(key, alpha))
    },
    stroke: (p, key, alpha) => {
      p.stroke(...rgba(key, alpha))
    },
    background: (p, key, alpha) => {
      p.background(...rgba(key, alpha))
    },
  }
}

export function painter(section: 'colors'): Painter<CoreKey>
export function painter<S extends SectionName>(section: S): Painter<SectionKey<S> & string>
export function painter(section: string): Painter<string> {
  return make(section === 'colors' ? '' : `${section}.`)
}

/** bumps on every theme change; for components that draw to a 2D canvas */
export const useThemeChange = (): number => {
  const [n, setN] = useState(0)
  useEffect(() => onThemeChange(() => setN(x => x + 1)), [])
  return n
}
