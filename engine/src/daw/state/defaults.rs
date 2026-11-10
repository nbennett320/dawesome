use crate::daw;

pub static SAMPLE_RATE: u32 = 44_100;
pub static TEMPO: f32 = 120.;
pub static MAX_BEATS: u64 = 16;
// the playlist's length: 64 bars of four beats. clips start within it
pub static MAX_BEATS_DISPLAYED: u64 = 256;
// with no clips, the loop is this many bars long
pub static EMPTY_LOOP_BARS: u32 = 4;
pub static SNAP_ENABLED: bool = true;
// snap to the beat, which lines up with the playlist grid
pub static SNAP_SUBDIVISION: daw::timing::Note = daw::timing::Note::QuarterNote;
pub static NUM_OF_TRACKS: u32 = 4;
