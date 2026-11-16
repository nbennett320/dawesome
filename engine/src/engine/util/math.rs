use std::cmp::PartialOrd;
use num_traits::{
  Num, 
  NumOps,
  PrimInt,
};

pub fn round_to_nearest_multiple<
  T: 
    // PrimInt +
    Num +
    NumOps +
    PartialOrd +
    From<U> +
    From<u8> +
    Into<T> +
    Copy +
    std::fmt::Display,
  U:
    PrimInt +
    Num +
    NumOps +
    PartialOrd +
    From<T> +
    Into<T> +
    Copy +
    std::fmt::Display
>(
  n: T,
  multiple: U,
) -> T {
  let denom: u8 = 2;
  ((n + multiple.into() / denom.into()) / multiple.into()) * multiple.into()
}

pub fn clamp_or(value: f32, range: std::ops::RangeInclusive<f32>, default: f32) -> f32 {
  if value.is_nan() { default } else { value.clamp(*range.start(), *range.end()) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_round_to_nearest_multiple() {
    let val: u32 = round_to_nearest_multiple(12 as u32, 5 as u32);
    assert_eq!(val, 10);

    let val: u32 = round_to_nearest_multiple(13 as u32, 5 as u32);
    assert_eq!(val, 15);

    let val: u32 = round_to_nearest_multiple(149 as u32, 10 as u32);
    assert_eq!(val, 150);

    let val: i32 = round_to_nearest_multiple::<i32, i32>(201, 10);
    assert_eq!(val, 200);

    let val: i32 = round_to_nearest_multiple::<i32, i32>(143, 12);
    assert_eq!(val, 144);

    // todo: make multiple unsigned
    let val: i32 = round_to_nearest_multiple::<i32, i32>(-1001, -100);
    assert_eq!(val, -1000);

    let val: u8 = round_to_nearest_multiple::<u8, u8>(127, 8);
    assert_eq!(val, 128);

    let val: i16 = round_to_nearest_multiple::<i16, i16>(127, 6);
    assert_eq!(val, 126);

    // // todo: handle floats
    // let val: f32 = round_to_nearest_multiple::<f32, i32>(99.9, 10);
    // assert_eq!(val, 126);
  }

  #[test]
  fn test_clamp_or() {
    assert_eq!(clamp_or(0.5, -1.0..=1.0, 0.), 0.5);
    assert_eq!(clamp_or(7., -1.0..=1.0, 0.), 1.);
    assert_eq!(clamp_or(-7., -1.0..=1.0, 0.), -1.);
    assert_eq!(clamp_or(f32::INFINITY, -1.0..=1.0, 0.), 1.);
    assert_eq!(clamp_or(f32::NAN, -1.0..=1.0, 0.25), 0.25);
  }
}
