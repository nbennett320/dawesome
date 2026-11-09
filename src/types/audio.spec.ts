import { AudioSettings, OutputStatus, bufferMs, bufferNote, meterPosition, msLabel, rateLabel, takeSummary } from './audio'

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

test('meter positions', () => {
  expect(meterPosition(1)).toBe(1)
  expect(meterPosition(0.001)).toBeCloseTo(0)
  expect(meterPosition(0.0316)).toBeCloseTo(0.5, 2)
  expect(meterPosition(0)).toBe(0)
  expect(meterPosition(NaN)).toBe(0)
  // clipping stays at the top
  expect(meterPosition(1.4)).toBe(1)
})

test('what a take says', () => {
  const take = { path: '/songs/song/recordings/song_11092026_0.wav', seconds: 3.04, channels: 1, dropped: 0 }
  expect(takeSummary(take)).toBe('Recorded song_11092026_0.wav (3.0s, mono)')
  expect(takeSummary({ ...take, channels: 2, dropped: 12 }))
    .toBe("Recorded song_11092026_0.wav (3.0s, stereo), but some audio was lost because the disk couldn't keep up")
})
