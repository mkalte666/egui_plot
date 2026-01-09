use std::cmp::Ordering;

use crate::{GridInput, GridMark};

/// Trait to describe an [AxisTransform] - a set of methods for handling the transformation of data from its own coordinate system to plot coordinates.
pub trait AxisTransform {
    /// Turn a data coordinate to normalized plot coordinates.
    fn data_to_plot(&self, data_bounds: [f64; 2], x: f64) -> f64;

    /// Turn a normalized plot coordinate into a data coordinate.
    fn plot_to_data(&self, data_bounds: [f64; 2], x: f64) -> f64;

    /// Grid spacer generation
    fn grid_marks(&self, input: &GridInput) -> Vec<GridMark>;
}

/// A linear transformation, that maps x -> sign * x, where sign is -1.0 if invert is true, and 1.0 otherwise.
pub struct LinearAxisTransform {
    invert: bool,
}

impl LinearAxisTransform {
    pub fn new(invert: bool) -> Self {
        Self { invert }
    }

    pub fn inverted() -> Self {
        Self::new(true)
    }

    pub fn normal() -> Self {
        Self::new(false)
    }

    #[inline]
    fn sign(&self) -> f64 {
        if self.invert { -1.0 } else { 1.0 }
    }
}

impl AxisTransform for LinearAxisTransform {
    #[inline]
    fn data_to_plot(&self, data_bounds: [f64; 2], x: f64) -> f64 {
        self.sign() * (x - data_bounds[0]) / (data_bounds[1] - data_bounds[0])
    }

    #[inline]
    fn plot_to_data(&self, data_bounds: [f64; 2], x: f64) -> f64 {
        data_bounds[0] + self.sign() * x * (data_bounds[1] - data_bounds[0])
    }

    fn grid_marks(&self, input: &GridInput) -> Vec<GridMark> {
        let log_base = 10.0;
        // handle degenerate cases
        if input.base_step_size.abs() < f64::EPSILON {
            return Vec::new();
        }

        // The distance between two of the thinnest grid lines is "rounded" up
        // to the next-bigger power of base
        let smallest_visible_unit = next_power(input.base_step_size, log_base);

        let step_sizes = [
            smallest_visible_unit,
            smallest_visible_unit * log_base,
            smallest_visible_unit * log_base * log_base,
        ];

        generate_marks(step_sizes, input.bounds)
    }
}

/// Returns next bigger power in given base
/// e.g.
/// ```ignore
/// use egui_plot::next_power;
/// assert_eq!(next_power(0.01, 10.0), 0.01);
/// assert_eq!(next_power(0.02, 10.0), 0.1);
/// assert_eq!(next_power(0.2,  10.0), 1);
/// ```
fn next_power(value: f64, base: f64) -> f64 {
    debug_assert_ne!(value, 0.0, "Bad input"); // can be negative (typical for Y axis)
    base.powi(value.abs().log(base).ceil() as i32)
}

/// Fill in all values between [min, max] which are a multiple of `step_size`
fn generate_marks(step_sizes: [f64; 3], bounds: (f64, f64)) -> Vec<GridMark> {
    let mut steps = vec![];
    fill_marks_between(&mut steps, step_sizes[0], bounds);
    fill_marks_between(&mut steps, step_sizes[1], bounds);
    fill_marks_between(&mut steps, step_sizes[2], bounds);

    // Remove duplicates:
    // This can happen because we have overlapping steps, e.g.:
    // step_size[0] =   10  =>  [-10, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120]
    // step_size[1] =  100  =>  [     0,                                     100          ]
    // step_size[2] = 1000  =>  [     0                                                   ]

    steps.sort_by(|a, b| cmp_f64(a.value, b.value));

    let min_step = step_sizes.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let eps = 0.1 * min_step; // avoid putting two ticks too closely together

    let mut deduplicated: Vec<GridMark> = Vec::with_capacity(steps.len());
    for step in steps {
        if let Some(last) = deduplicated.last_mut() {
            if (last.value - step.value).abs() < eps {
                // Keep the one with the largest step size
                if last.step_size < step.step_size {
                    *last = step;
                }
                continue;
            }
        }
        deduplicated.push(step);
    }

    deduplicated
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    match a.partial_cmp(&b) {
        Some(ord) => ord,
        None => a.is_nan().cmp(&b.is_nan()),
    }
}

/// Fill in all values between [min, max] which are a multiple of `step_size`
fn fill_marks_between(out: &mut Vec<GridMark>, step_size: f64, (min, max): (f64, f64)) {
    debug_assert!(min <= max, "Bad plot bounds: min: {min}, max: {max}");
    let first = (min / step_size).ceil() as i64;
    let last = (max / step_size).ceil() as i64;

    let marks_iter = (first..last).map(|i| {
        let value = (i as f64) * step_size;
        GridMark { value, step_size }
    });
    out.extend(marks_iter);
}
