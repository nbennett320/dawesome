use std::fmt::Debug;
use std::vec::{Vec};
use std::cmp::PartialOrd;
use num_traits::{
  Num, 
  NumOps,
  NumCast,
  PrimInt,
  Float,
};
use std::iter::{
  Sum,
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

pub fn f_normalize<
  T: 
    Num +
    Into<f32> +
    Float +
    Debug
>(
  xs: Vec<T>,
) -> Vec<f32> {
  let xsf: Vec<f32> = xs.iter().map(|x| (x.to_f32().unwrap())).collect();
  let min = xsf
    .iter()
    .enumerate()
    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    .unwrap().1;
  let max = xsf
    .iter()
    .enumerate()
    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
    .unwrap().1;

  // all the same (like silence): flat, rather than dividing by zero
  if max == min { return vec![0.; xsf.len()]; }

  let mut norms = Vec::<f32>::new();
  for x in &xsf {
    let norm = (x - min) / (max - min);
    norms.push(norm);
  }

  norms
}

pub fn gaussian_1d<
  T:
    Num +
    NumCast +
    Copy +
    Sized +
    Sum
>(
  xs: &Vec<T>,
  radius: f32,
  only_weights: bool,
) -> Option<Vec<f32>> {
  match(std_dev(xs), xs.len()) {
    // a constant signal (like silence) has nothing to smooth, and would
    // divide by zero below
    (Some(sd), len) if len > 0 && sd == 0. => Some(xs
      .iter()
      .map(|x| if only_weights { 1. } else { x.to_f32().unwrap() })
      .collect()),
    (Some(sd), len) if len > 0 => {
      use std::f32::consts::{PI, E};
      
      let sdf = sd * radius;
      let mut gx = Vec::new();

      for x in xs {
        let x_sq = x.to_f32().unwrap() * x.to_f32().unwrap();
        let exponent = -1. * (x_sq) / (2. * sdf * sdf);
        let gxi = (1. / (2. * PI * sdf.powi(2)).sqrt()) * E.powf(exponent);

        if only_weights {
          gx.push(gxi);
        } else {
          gx.push(gxi * x.to_f32().unwrap());
        }
      }

      Some(gx)
    },
    _ => None
  }
}

pub fn std_dev<
  T: 
    Num + 
    NumCast +
    Copy +
    Sized +
    Sum
>(
  xs: &Vec<T>
) -> Option<f32> {
  match(mean(xs), xs.len()) {
    (Some(xs_mean), len) if len > 0 => {
      let variance = xs.iter().map(|val| {
        let diff = xs_mean - (val.to_f32().unwrap());

        diff * diff
      }).sum::<f32>() / len as f32;

      Some(variance.sqrt())
    },
    _ => None
  }
}

pub fn mean<
  T:
    Num + 
    NumCast +
    Copy +
    Sized +
    Sum
>(
  xs: &Vec<T>
) -> Option<f32> {
  let sum = xs
    .iter()
    .copied()
    .sum::<T>()
    .to_f32()
    .unwrap();
  
  match xs.len() {
    positive if positive > 0 => Some(sum / xs.len() as f32),
    _ => None
  }
}

pub fn sample_to_n_elements<
  T:
    Num + 
    NumCast +
    Copy +
    Sized +
    Sum
>(
  xs: &Vec<T>,
  n: u32,
) -> Option<Vec<f32>> {
  match(xs, xs.len()) {
    (xs, len) if (n as usize) < len => {
      let chunk_size = xs.len() / n as usize;
      println!("chunk_size: {}", chunk_size);
      let chunks = xs.chunks(chunk_size);
      let mut res: Vec<f32> = chunks.map(|chunk| { mean(&chunk.to_vec()).unwrap() }).collect();
      println!("reduced len: {}", res.len());

      while res.len() > n as usize {
        res.pop();
      }

      Some(res)
    }
    _ => None
  }
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
  fn test_gaussian_1d() {
    let xs: Vec<f32> = vec![12.4, -3.321, 3.143222, 593.098];
    let radius = 1.;

    let gauss = gaussian_1d(&xs, radius, false);
    assert_eq!(gauss.unwrap(), [0.019367829, -0.0051928335, 0.004914897, 0.06218239]);

    let radius = 0.001;
    let gauss = gaussian_1d(&xs, radius, false);
    assert_eq!(gauss.unwrap(), [0.0, -8.281204e-37, 5.3503775e-33, 0.0]);
  }

  #[test]
  fn test_sample_to_n_elements() {
    let xs: Vec<f32> = vec![4.32, 522.345, -9.284, 1234.542, 1.023, -2.003, 3.1423, 2.];    

    let interp = sample_to_n_elements(&xs, 4);
    assert_eq!(interp.unwrap().len(), 4);

    let interp = sample_to_n_elements(&xs, 3);
    assert_eq!(interp.unwrap().len(), 3);
  }

  #[test]
  fn test_f_normalize() {
    let xs: Vec<f32> = vec![4.32, 522.345, -9.284, 1234.542, 1.023, -2.003, 3.1423, 2.];    
    let normalized = f_normalize(xs);

    let mut greater_than_one = Vec::<f32>::new();
    for x in normalized {
      if x.abs() > 1. {
        greater_than_one.push(x);
      }
    }

    assert_eq!(greater_than_one.len(), 0);
  }
}
