// a parameter for tests, as the engine would describe it
import { ParamWire } from './nativeEffect'

export const param = (id: number, change: Partial<ParamWire> = {}): ParamWire => ({
  id,
  key: `p${id}`,
  name: `P${id}`,
  group: 'Main',
  unit: '',
  min: 0,
  max: 1,
  default: 0,
  defaultNormalized: 0,
  taper: 'linear',
  stepCount: 0,
  labels: [],
  value: { id, value: 0, normalized: 0, text: '0.00' },
  ...change,
})
