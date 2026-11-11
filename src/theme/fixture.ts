/// <reference types="node" />
import * as fs from 'fs'
import * as path from 'path'
import type { ResolvedTheme } from './keys'

// read with fs: jest's JSON default import is undefined without esModuleInterop
const file = path.resolve(__dirname, '../../engine/themes/test/slate-light.resolved.json')

export const slateLightTheme: ResolvedTheme = {
  id: 'slate-light',
  name: 'Slate Light',
  appearance: 'light',
  colors: JSON.parse(fs.readFileSync(file, 'utf8')),
}
