// the rust network against the training tools' output for the same input
// (tools/tape-model writes both files: `model.json` and a fixture). they
// must agree within 1e-5 (G5's T1, and C1 in the plan)

use std::fs;
use std::sync::Arc;

use serde_json::Value;
use tape::{Gru, GruWeights};

fn floats(v: &Value) -> Vec<f32> {
  v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect()
}

// what the DAW's glue will do: model.json into plain arrays
fn weights(spec: &Value) -> GruWeights {
  assert_eq!(spec["schema"], 1);
  assert_eq!(spec["architecture"], "gru");
  let w = &spec["weights"];
  GruWeights {
    hidden: spec["hidden"].as_u64().unwrap() as usize,
    w_ih: floats(&w["w_ih"]["values"]),
    w_hh: floats(&w["w_hh"]["values"]),
    b_ih: floats(&w["b_ih"]["values"]),
    b_hh: floats(&w["b_hh"]["values"]),
    dense_w: floats(&w["dense_w"]["values"]),
    dense_b: floats(&w["dense_b"]["values"])[0],
    drive_scale: spec["inputs"][1]["scale"].as_f64().unwrap() as f32,
    rate: spec["rate"].as_u64().unwrap() as u32,
  }
}

fn load(name: &str) -> Value {
  let path = format!("{}/tests/assets/{name}", env!("CARGO_MANIFEST_DIR"));
  serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn matches_the_training_tools() {
  let spec = load("fake.model.json");
  let fixture = load("fake.fixture.json");
  let weights = Arc::new(weights(&spec));
  assert_eq!(weights.rate, 48_000);
  let cases = fixture["cases"].as_array().unwrap();
  assert!(cases.len() >= 2);
  for case in cases {
    let drive = case["drive_db"].as_f64().unwrap() as f32;
    let input = floats(&case["input"]);
    let want = floats(&case["output"]);
    let mut gru = Gru::new(weights.clone()).unwrap();
    let mut got = vec![0.0; input.len()];
    gru.process(&input, drive, &mut got);
    let worst = got.iter().zip(&want).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    assert!(worst <= 1e-5, "drive {drive}: off by {worst}");
  }
}
