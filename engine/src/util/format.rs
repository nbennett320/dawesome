use std::time::{Duration};

// a length of time as m:ss:ms
pub fn format_runtime(dur: Duration) -> String {

  let format = || -> String {
    let m = (dur.as_millis() as f32 / 60_000.).floor() as i64;
    let s = ((dur.as_millis() - (m * 60_000) as u128) as f32 / 1_000.).floor() as i64;
    let ms = dur.as_millis() - (m * 60_000) as u128 - (s * 1_000) as u128;
    let mbufc: &str = if m < 10 { "0" } else { "" };
    let sbufc: &str = if s < 10 { "0" } else { "" };
    let msbufc: &str = if ms < 10 { "0" } else { "" };
    format!("{sbufc}{m}:{mbufc}{s}:{msbufc}{ms}")
  };

  format()
}
