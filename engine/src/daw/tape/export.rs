// the tape in an export (T37): a mixdown sounds as the tape does, not as
// the tracks would without it. its engine gets a tape of its own, built as
// playback's is (live.rs), at the export's rate
//
// - print mode: each taped track plays its print, as it does in playback.
//   a print that's in and still right (made from what's in the export, at
//   its rate, with its drive) is used as it is; any other is made now, from
//   the export's copy of the song. the export starts on its prints, not
//   crossing in from direct
// - a track print mode can't print yet (one playing an instrument, or with
//   the tape after its effects: D76) is recorded live in the export, as
//   always-live mode plays it. (it's a head's delay late, 8 ms, against the
//   printed ones: O7)
// - always-live mode: every taped track recorded live, as heard
//
// remove the print branch (and print.rs) to drop print mode: see
// context/tape-project/08-mode-removal.md
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tape::MachineSettings;

use super::{live, print};
use crate::daw::audiograph::AudioGraph;
use crate::daw::tape_settings::TapeSettings;
use crate::engine::tape::{EngineTape, Print};
use crate::preferences::{TapeMethod, TapeMode, TapeTap};

// the tape as it was when the export started
pub struct Preferences {
  pub settings: TapeSettings,
  pub mode: TapeMode,
  pub tap: TapeTap,
  pub method: TapeMethod,
  // the prints made so far: (content, recipe, print) by track
  pub prints: BTreeMap<u32, (u64, print::Recipe, Arc<Print>)>,
}

// the tape for an export's engine, and its knobs (the tail fades the hiss
// out with them)
pub struct ExportTape {
  pub tape: Box<EngineTape>,
  pub settings: MachineSettings,
}

// the tracks whose prints have to be made for the export (print mode),
// in order: to say how far along it is
pub fn prints_to_make(tape: &Preferences, song: &AudioGraph<'static>, signature: &dyn std::fmt::Debug, rate: u32) -> Vec<u32> {
  let Some(shape) = live::shape(&tape.settings, tape.mode, tape.tap, tape.method, rate) else { return Vec::new() };
  if shape.live || !shape.before_effects {
    return Vec::new();
  }
  shape.tracks.iter().copied().filter(|&track| reusable(tape, song, signature, track, rate).is_none()).collect()
}

// a print that's in and right for the export, if there is one
fn reusable(tape: &Preferences, song: &AudioGraph<'static>, signature: &dyn std::fmt::Debug, track: u32, rate: u32) -> Option<Arc<Print>> {
  let (content, recipe, print) = tape.prints.get(&track)?;
  let wanted = recipe_for(tape, track, rate);
  (*recipe == wanted && *content == print::content_of(song, signature, track) && print.rate == rate).then(|| print.clone())
}

fn recipe_for(tape: &Preferences, track: u32, rate: u32) -> print::Recipe {
  print::Recipe {
    rate,
    drive_db: tape.settings.track(track).drive_db,
    dbx: tape.settings.machine.dbx,
    neural: tape.method == TapeMethod::Neural,
  }
}

// the export's tape, or None when nothing's on tape. `made` is told after
// each print it makes
pub fn build(
  tape: &Preferences,
  song: &AudioGraph<'static>,
  signature: &dyn std::fmt::Debug,
  rate: u32,
  cancel: &AtomicBool,
  made: &mut dyn FnMut(),
) -> Result<Option<ExportTape>, String> {
  let Some(shape) = live::shape(&tape.settings, tape.mode, tape.tap, tape.method, rate) else { return Ok(None) };

  // print mode: each track's print, or live if it can't have one
  let mut prints = BTreeMap::new();
  if !shape.live && shape.before_effects {
    for &track in &shape.tracks {
      let print = match reusable(tape, song, signature, track, rate) {
        Some(print) => Some(print),
        None => {
          let recipe = recipe_for(tape, track, rate);
          let made_now = match print::render_pre_tape_of(song, track, rate, cancel) {
            Ok(mix) => Some(Arc::new(print::make_print(&mix, &recipe, cancel)?)),
            Err(e) if e == print::NEEDS_PLUGINS => None,
            Err(e) => return Err(e),
          };
          made();
          made_now
        }
      };
      if let Some(print) = print {
        prints.insert(track, print);
      }
    }
  }

  let mut built = live::build_with(&shape, &tape.settings, &|track| !prints.contains_key(&track));
  for (&track, print) in &prints {
    built.set_print(track, Some(print.clone()));
  }
  built.begin_on_prints();
  Ok(Some(ExportTape { tape: built, settings: live::machine_settings(&tape.settings) }))
}
