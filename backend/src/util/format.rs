pub fn format_playlist_runtime(start_ms: i64) -> String {
  let now_ts = chrono::offset::Local::now().timestamp();
  let dur = now_ts - start_ms;

  let format = || -> String {
    let m = (dur as f32 / 60_000.).floor() as i64;
    let s = ((dur - (m * 60_000)) as f32 / 1_000.).floor() as i64;
    let ms = dur - (m * 60_000) - (s * 1_000);
    let mbufc: &str = if m < 10 { "0" } else { "" };
    let sbufc: &str = if s < 10 { "0" } else { "" };
    let msbufc: &str = if ms < 10 { "0" } else { "" };
    format!("{sbufc}{m}:{mbufc}{s}:{msbufc}{ms}")
  };

  format()
}
