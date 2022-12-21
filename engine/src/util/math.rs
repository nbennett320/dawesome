use std::fmt::Debug;
use std::vec::{Vec};
use std::ops::{
  Add,
  Sub,
  Mul,
  Div
};
use std::cmp::{
  Ord,
  PartialOrd
};
use num_traits::{
  Num, 
  NumOps,
  NumCast,
  PrimInt,
  Signed,
  Unsigned,
  Float, ToPrimitive,
};
use rodio::cpal::Sample;
use std::iter::{
  Sum,
};

fn y0_prime<
  T:
    Add<Output = T> + 
    Sub<Output = T> + 
    Mul<Output = T> + 
    Div<Output = T> + 
    Copy +
    Into<T> +
    From<i32>,
  U:
    Add<Output = U> + 
    Sub<Output = U> + 
    Mul<Output = U> + 
    Div<Output = U> + 
    Copy +
    Into<T> +
    From<i32>
>(
  y0: T,
  x0: U,
  x1: U,
) -> T {
  y0 / (x0 - x1).into()
}

pub fn local_extremes<
  T: 
    Add<Output = T> + 
    Sub<Output = T> + 
    Mul<Output = T> + 
    Div<Output = T> + 
    Ord +
    PartialOrd +
    Copy +
    Into<U> +
    From<U> +
    From<i32>,
  U:
    Add<Output = U> + 
    Sub<Output = U> + 
    Mul<Output = U> + 
    Div<Output = U> + 
    Ord +
    PartialOrd +
    Copy +
    Into<T> +
    From<T> +
    From<i32>,
>(
  xs: Vec<T>,
  ys: Vec<U>,
) -> Option<(Vec<T>, Vec<U>)> {
  // verify lengths are the same 
  if xs.len() != ys.len() {
    return None;
  }

  let len = xs.len();

  let mut x_roots = Vec::new();
  let mut y_roots = Vec::new();

  for i in 1..len-2 {
    // append local maximums
    if ys[i-1] < ys[i] && ys[i] > ys[i+1] {
      x_roots.push(xs[i]);
      y_roots.push(ys[i]);
    }

    // append local minimums
    if ys[i-1] > ys[i] && ys[i] < ys[i+1] {
      x_roots.push(xs[i]);
      y_roots.push(ys[i]);
    }
  }

  Some((x_roots, y_roots))
}

fn numeric_derive<
  T: 
    Add<Output = T> + 
    Sub<Output = T> + 
    Mul<Output = T> + 
    Div<Output = T> + 
    Copy +
    Into<U> +
    From<U> +
    From<i32>,
  U:
    Add<Output = U> + 
    Sub<Output = U> + 
    Mul<Output = U> + 
    Div<Output = U> + 
    Copy +
    Into<T> +
    From<T> +
    From<i32>,
>(
  xs: Vec<T>,
  ys: Vec<U>,
) -> Option<(Vec<T>, Vec<U>)> {
  // verify lengths are the same 
  if xs.len() != ys.len() {
    return None;
  }

  let len = xs.len();

  let mut x_dx = Vec::new();
  let mut y_dx = Vec::new();

  for i in 1..len {
    let x0 = (i-1) as i32;
    let y0 = ys[i-1];
    let x1 = i as i32;
    let y1 = y0_prime(y0, x0, x1);

    x_dx.push(x0.into());
    y_dx.push(y1);
  }

  Some((x_dx, y_dx))
}

pub fn interpolate<
  T: 
    Add<Output = T> + 
    Sub<Output = T> + 
    Mul<Output = T> + 
    Div<Output = T> + 
    Copy +
    Into<U> +
    From<U> +
    From<i32>,
  U:
    Add<Output = U> + 
    Sub<Output = U> + 
    Mul<Output = U> + 
    Div<Output = U> + 
    Copy +
    Into<T> +
    From<T> +
    From<i32>,
>(
  xs: Vec<T>, 
  ys: Vec<U>,
) -> Option<(Vec<T>, Vec<U>)> {
  // verify lengths are the same 
  if xs.len() != ys.len() {
    return None;
  }

  let len = xs.len();

  // linear interpolation function
  let f = |p0: (T, U), p1: (T, U), xp: T| -> U {
    let (x0, y0) = p0;
    let (x1, y1) = p1;

    y0 + ( ( ( y1 - y0 ) / ( x1 - x0 ).into() ) ) * ( xp - x0 ).into()
  };

  let mut x_interp = Vec::new();
  let mut y_interp = Vec::new();
  
  let mut xp = 0;
  for i in 1..len-1 {
    let p0 = (((i-1) as i32).into(), ys[i-1]);
    let p1 = ((i as i32).into(), ys[i]);

    let yp = f(p0, p1, (i as i32).into());

    xp += 1;
    x_interp.push(xp.into());
    y_interp.push(yp);
  }

  Some((x_interp, y_interp))
}

pub fn interpolate_to<
  T:
    Add<Output = T> + 
    Sub<Output = T> + 
    Mul<Output = T> + 
    Div<Output = T> + 
    Ord + 
    Copy +
    Into<U> +
    Into<i32> +
    From<U> +
    From<i32>,
  U:
    Add<Output = U> + 
    Sub<Output = U> + 
    Mul<Output = U> + 
    Div<Output = U> + 
    Ord +
    Copy +
    Into<T> +
    Into<i32> +
    From<T> +
    From<i32>,
  V:
    Add<Output = V> + 
    Sub<Output = V> + 
    Mul<Output = V> + 
    Div<Output = V> + 
    Ord +
    Copy +
    Into<T> +
    Into<i32>,
>(
  xs: Vec<T>, 
  ys: Vec<U>,
  n: V,
) -> Option<(Vec<T>, Vec<U>)> {
  // verify lengths are the same 
  if xs.len() != ys.len() {
    println!("unequal lengths!");
    return None;
  }

  // return original vectors if their length
  // is less than or equal to the 
  // desired interpolation result
  if ys.len() as i32 <= n.into() {
    println!("why are you doing this!");
    return Some((xs, ys))
  }

  let mut xns = Vec::from(xs);
  let mut yns = Vec::from(ys);

  let mut i = yns.len();
  while i as i32 > n.into() {
    // println!("i: {}, {}, max: {:?}", i, xns.len(), yns.iter().max());
    (xns, yns) = interpolate(xns, yns).unwrap();
    i -= 1;
    // println!("i: {}, {}, max: {:?}", i, xns.len(), yns.iter().max());
  }

  Some((xns, yns))
}

pub fn round_to_nearest_signed_multiple<
  T: 
    Num +
    NumOps +
    Signed +
    PartialOrd +
    From<U> + 
    Copy,
  U:
    PrimInt +
    Unsigned
>(
  n: T,
  multiple: U,
) -> T {
  let remainder = n.abs() % multiple.into();
  let ceil = n + multiple.into() - remainder;
  let floor = n - multiple.into() + remainder;

  if multiple.is_zero() {
    return n;
  }

  if remainder.is_zero() {
    return n;
  }

  // if n.is_negative() {
  //   return -(n.abs() - remainder);
  // }

  let ceil_diff = (n - ceil).abs();
  let floor_diff = (n - floor).abs();

  if ceil_diff <= floor_diff { ceil } else { floor }
}

pub fn round_to_nearest_unsigned_multiple<
  T: 
    PrimInt +
    Num +
    NumOps +
    Unsigned +
    PartialOrd +
    From<U> +
    Copy +
    std::fmt::Display,
  U:
    PrimInt +
    Unsigned +
    From<T> +
    std::fmt::Display
>(
  n: T,
  multiple: U,
) -> T {
  println!("rounding to nearest unsigned mult: {}, {}", n, multiple);
  let remainder = n % multiple.into();
  let mult: T = multiple.into();
  println!("calculatd remainder: {}", remainder);
  let ceil = n + multiple.into() - remainder;
  let floor = n - multiple.into() + (mult - remainder);

  if multiple.is_zero() {
    return n;
  }

  if remainder.is_zero() {
    return n;
  }

  let ceil_diff = (n + n) - ceil;
  let floor_diff = (n + n) - floor;

  println!("ceil: {}, floor: {}", ceil, floor);

  if ceil_diff <= floor_diff { ceil } else { floor }
}

pub fn vec_itof32 <
  T:
    PrimInt +
    Num
>(
  xs: Vec<T>
) -> Vec<f32> {
  xs
    .iter()
    .map(|x| x.to_f32().unwrap())
    .collect()
}

pub fn vec_itof64 <
  T:
    PrimInt +
    Num
>(
  xs: Vec<T>
) -> Vec<f64> {
  xs
    .iter()
    .map(|x| x.to_f64().unwrap())
    .collect()
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

  let mut norms = Vec::<f32>::new();
  for x in &xsf {
    let norm = (x - min) / (max - min);
    norms.push(norm);
  }

  norms
}

pub fn interleave<
  T:
    Copy +
    Num
>(
  xs: Vec<T>,
  ys: Vec<T>,
) -> Option<Vec<T>> {
  if xs.len() != ys.len() {
    println!("Error interleaving vectors: unequal lengths");
    return None
  }
  
  let len = xs.len();
  let mut zs = Vec::<T>::new();

  for i in 0..len {
    zs.push(xs[i]);
    zs.push(ys[i]);
  }

  Option::Some(zs)
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
