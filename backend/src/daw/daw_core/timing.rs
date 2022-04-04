pub fn tempo_to_intrv_ms(tempo: f32) -> u64 {
  let min_ms = 60_000.;
  let ms_intrv = min_ms / tempo;
  ms_intrv.round() as u64
}
