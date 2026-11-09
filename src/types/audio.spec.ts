import {
  AudioSettings, OutputStatus, bufferMs, bufferNote, defaultInput, fitInput, inputChoices, inputKey, inputLabel, meterPosition,
  msLabel, parseInputKey, rateLabel,
} from './audio'

const output = (bufferFrames: number): OutputStatus => ({
  device: 'AirPods', sampleRate: 48_000, bufferFrames, latencyMs: 5.3,
})
const settings = (bufferFrames: number | null): AudioSettings => ({
  outputDevice: null, inputDevice: null, bufferFrames, recordOffsetMs: 0,
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

test('meter positions', () => {
  expect(meterPosition(1)).toBe(1)
  expect(meterPosition(0.001)).toBeCloseTo(0)
  expect(meterPosition(0.0316)).toBeCloseTo(0.5, 2)
  expect(meterPosition(0)).toBe(0)
  expect(meterPosition(NaN)).toBe(0)
  // clipping stays at the top
  expect(meterPosition(1.4)).toBe(1)
})

test('choices fit the input, as the engine fits them', () => {
  const mono = (first: number) => ({ first, stereo: false })
  const pair = (first: number) => ({ first, stereo: true })

  expect(fitInput(pair(0), 2)).toEqual(pair(0))
  expect(fitInput(pair(2), 4)).toEqual(pair(2))
  expect(fitInput(mono(3), 4)).toEqual(mono(3))
  // the MacBook's microphone has one channel
  expect(fitInput(pair(0), 1)).toEqual(mono(0))
  expect(fitInput(pair(2), 2)).toEqual(pair(0))
  expect(fitInput(mono(5), 4)).toEqual(mono(0))
  // before the input's channels are known
  expect(fitInput(pair(4), null)).toEqual(pair(4))
})

test('input choices', () => {
  expect(defaultInput(1)).toEqual({ first: 0, stereo: false })
  expect(defaultInput(8)).toEqual({ first: 0, stereo: true })
  expect(inputChoices(3).map(inputLabel)).toEqual(['In 1', 'In 2', 'In 3', 'In 1+2'])
  expect(inputChoices(4).map(inputLabel)).toEqual(['In 1', 'In 2', 'In 3', 'In 4', 'In 1+2', 'In 3+4'])

  const pair = { first: 2, stereo: true }
  expect(parseInputKey(inputKey(pair))).toEqual(pair)
  expect(parseInputKey('nope')).toBeNull()
})
