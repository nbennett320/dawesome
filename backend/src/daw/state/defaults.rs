use crate::daw;

pub static SAMPLE_RATE: u32 = 44_100;
pub static TEMPO: f32 = 120.;
pub static MAX_BEATS: u64 = 32;
pub static MAX_BEATS_DISPLAYED: u64 = 16;
pub static SNAP_ENABLED: bool = false;
pub static SNAP_SUBDIVISION: daw::timing::Note = daw::timing::Note::WholeNote;
