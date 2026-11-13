use crate::daw;

pub static SAMPLE_RATE: u32 = 44_100;
pub static TEMPO: f32 = 120.;
pub static MAX_BEATS: u64 = 16;
// the playlist's length, in bars of its time signature. clips start within
// it. it's longer when clips reach further (see daw::fit_playlist)
pub static PLAYLIST_BARS: u64 = 128;
// with no clips, the loop is this many bars long
pub static EMPTY_LOOP_BARS: u32 = 4;
pub static SNAP_ENABLED: bool = true;
// snap to the beat, which lines up with the playlist grid
pub static SNAP_SUBDIVISION: daw::timing::Note = daw::timing::Note::QuarterNote;
pub static NUM_OF_TRACKS: u32 = 4;
