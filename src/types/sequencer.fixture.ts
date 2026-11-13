// sequencer things for the specs, with what doesn't matter to a spec filled
// in. a synth row's slot is never its id, so a spec that mixes them up fails.
// each one made has a rev of its own, as the engine gives one to everything
// it makes or changes (so the slice takes it in place of what it had)
import {
  DEFAULT_PITCH, Note, Pattern, Row, STEP_VELOCITY, SampleInstrument, SequencerState, SynthInstrument,
} from './sequencer'

let revs = 0
// a rev no fixture has had, as the engine gives a row or pattern it changed
export const freshRev = () => {
  revs += 1
  return revs
}

export const sampleRow = (id: number, more: Partial<Row> & Partial<Omit<SampleInstrument, 'kind'>> = {}): Row<SampleInstrument> => {
  const { path = `/drums/${id}.wav`, samplePitch = DEFAULT_PITCH, gated = false, ...row } = more
  return {
    id,
    rev: freshRev(),
    label: null,
    volume: 1,
    pan: 0,
    muted: false,
    stepPitch: DEFAULT_PITCH,
    ...row,
    instrument: { kind: 'sample', path, samplePitch, gated },
  }
}

// its slot is 100 more than its id, unless given
export const synthRow = (id: number, more: Partial<Row> & Partial<Omit<SynthInstrument, 'kind'>> = {}): Row<SynthInstrument> => {
  const {
    slot = id + 100, pluginId = `vst3:${id}`, name = `Synth ${id}`, vendor = 'Acme', missing = false, hasEditor = true,
    ...row
  } = more
  return {
    id,
    rev: freshRev(),
    label: null,
    volume: 1,
    pan: 0,
    muted: false,
    stepPitch: DEFAULT_PITCH,
    ...row,
    instrument: { kind: 'synth', slot, pluginId, name, vendor, missing, hasEditor },
  }
}

// an unnamed 1-bar pattern with no notes, unless given
export const pattern = (id: number, more: Partial<Pattern> = {}): Pattern => {
  const length = more.length ?? 16
  return { id, rev: freshRev(), name: null, length, floor: length, notes: {}, ...more }
}

// a clicked step's note
export const step = (start: number, pitch = DEFAULT_PITCH): Note =>
  ({ pitch, start, length: 1, velocity: STEP_VELOCITY })

export const steps = (...starts: number[]): Note[] => starts.map(start => step(start))

export const sequencerOf = (more: Partial<SequencerState> = {}): SequencerState =>
  ({ rows: [], patterns: [], folders: [], ...more })
