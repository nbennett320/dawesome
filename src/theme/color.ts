export type RGBA = [number, number, number, number]

/** '#rgb' | '#rrggbb' | '#rrggbbaa' to [r, g, b, a], each 0-255 */
export const parseHex = (hex: string): RGBA => {
  let h = hex.trim().replace(/^#/, '')
  if(h.length === 3) h = h.replace(/./g, c => c + c)
  if(h.length === 6) h += 'ff'
  if(!/^[0-9a-fA-F]{8}$/.test(h)) throw new Error(`bad hex color: ${hex}`)
  const n = (i: number) => parseInt(h.slice(i, i + 2), 16)
  return [n(0), n(2), n(4), n(6)]
}

/** '#rrggbb' to 'r g b', for `rgb(var(--c-x) / a)` */
export const channels = (hex: string): string => {
  const [r, g, b] = parseHex(hex)
  return `${r} ${g} ${b}`
}

// `amount` of a over b, per channel, alpha included
export const mixRgba = (a: Readonly<RGBA>, b: Readonly<RGBA>, amount: number): RGBA =>
  [0, 1, 2, 3].map(i => Math.round(a[i] * amount + b[i] * (1 - amount))) as RGBA
