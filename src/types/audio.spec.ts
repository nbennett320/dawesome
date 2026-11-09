import { AudioSettings, OutputStatus, bufferMs, bufferNote, msLabel, rateLabel } from './audio'

const output = (bufferFrames: number): OutputStatus => ({
  device: 'AirPods', sampleRate: 48_000, bufferFrames, latencyMs: 5.3,
})
const settings = (bufferFrames: number | null): AudioSettings => ({
  outputDevice: null, inputDevice: null, bufferFrames,
})

test('buffer lengths', () => {
  expect(bufferMs(256, 48_000)).toBeCloseTo(5.333)
  expect(bufferMs(256, 0)).toBe(0)
})

test('labels', () => {
  expect(msLabel(5.333)).toBe('5.3 ms')
  expect(msLabel(21.33)).toBe('21 ms')
  expect(rateLabel(48_000)).toBe('48 kHz')
  expect(rateLabel(44_100)).toBe('44.1 kHz')
})

test('a buffer the device would not take is explained', () => {
  expect(bufferNote(settings(128), output(256))).toBe("Set to 128, but AirPods won't go lower than 256")
  expect(bufferNote(settings(512), output(512))).toBeNull()
  // the device's own default, or nothing known yet
  expect(bufferNote(settings(null), output(256))).toBeNull()
  expect(bufferNote(settings(128), output(0))).toBeNull()
  expect(bufferNote(settings(128), null)).toBeNull()
})
