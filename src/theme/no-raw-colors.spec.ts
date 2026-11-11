/// <reference types="node" />
import * as fs from 'fs'
import * as path from 'path'

// every color comes from the theme (engine/themes/base.json): use a token
// class (bg-surface), cssVar('ink') in inline styles, or painter(section) on
// canvases. only src/theme/ may hold raw colors
const SRC = path.resolve(__dirname, '..')
const THEME_DIR = __dirname

const patterns: RegExp[] = [
  /\b(bg|text|border|ring|ring-offset|from|via|to|fill|stroke|divide|outline|placeholder|accent|caret|decoration|shadow)-(slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}\b/,
  /\b(bg|text|border|ring|fill|stroke)-(white|black)\b/,
  /#[0-9a-fA-F]{3,8}\b/,
  /rgba?\(\s*\d/,
  /hsla?\(/,
  /GrayText/,
  /\.(fill|stroke|background)\(\s*\d/,
]

const walk = (dir: string): string[] =>
  fs.readdirSync(dir, { withFileTypes: true }).flatMap(e => {
    const full = path.join(dir, e.name)
    if(e.isDirectory()) return full === THEME_DIR ? [] : walk(full)
    return /\.(ts|tsx|scss|css)$/.test(e.name) ? [full] : []
  })

export const findRawColors = (): string[] =>
  walk(SRC).flatMap(file =>
    fs
      .readFileSync(file, 'utf8')
      .split('\n')
      .flatMap((line, i) =>
        patterns.some(re => re.test(line))
          ? [`${path.relative(SRC, file)}:${i + 1}: ${line.trim()}`]
          : [],
      ),
  )

describe('no raw colors outside src/theme', () => {
  it('has none', () => {
    expect(findRawColors()).toEqual([])
  })
})
