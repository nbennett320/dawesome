use std::vec::Vec;
use std::ops::{
  Add,
  Sub,
  Mul,
  Div
};
use std::cmp::{
  Ord,
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
