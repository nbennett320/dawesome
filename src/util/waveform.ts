// a waveform's levels as the engine sends them (see encode_levels in
// engine/src/daw/daw_core/waveform.rs): each level a little-endian u16 in
// base64, 0..65535 standing for 0..1
export const decodeLevels = (encoded: string): Float32Array => {
  const bytes = atob(encoded)
  const levels = new Float32Array(Math.floor(bytes.length / 2))

  for(let i = 0; i < levels.length; i++) {
    levels[i] = (bytes.charCodeAt(2 * i) + bytes.charCodeAt(2 * i + 1) * 256) / 65535
  }

  return levels
}
