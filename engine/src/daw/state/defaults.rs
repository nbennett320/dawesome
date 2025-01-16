use crate::daw;

pub static SAMPLE_RATE: u32 = 44_100;
pub static TEMPO: f32 = 120.;
pub static MAX_BEATS: u64 = 16;
pub static MAX_BEATS_DISPLAYED: u64 = 16;
pub static SNAP_ENABLED: bool = true;
pub static SNAP_SUBDIVISION: daw::timing::Note = daw::timing::Note::WholeNote;
pub static NUM_OF_TRACKS: u32 = 4;
