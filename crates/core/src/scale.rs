//! Pixel to real-length conversion.

use std::fmt;

use crate::Unit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleError {
    /// The pixel length or the entered length is zero, negative, NaN or infinite.
    NonPositive,
    /// The unit is not a physical unit (pixels cannot be calibrated).
    NotPhysical,
}

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaleError::NonPositive => f.write_str("lengths must be greater than zero"),
            ScaleError::NotPhysical => f.write_str("calibration needs a physical unit"),
        }
    }
}

impl std::error::Error for ScaleError {}

/// How many image pixels make one real unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pixels_per_unit: f64,
    unit: Unit,
}

impl Scale {
    /// Scale from a line of `pixel_length` pixels that is `entered_length` `unit`s long.
    pub fn from_calibration(
        pixel_length: f64,
        entered_length: f64,
        unit: Unit,
    ) -> Result<Self, ScaleError> {
        if !unit.is_physical() {
            return Err(ScaleError::NotPhysical);
        }
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(pixel_length) || !positive(entered_length) {
            return Err(ScaleError::NonPositive);
        }
        let pixels_per_unit = pixel_length / entered_length;
        if !positive(pixels_per_unit) {
            return Err(ScaleError::NonPositive);
        }
        Ok(Self {
            pixels_per_unit,
            unit,
        })
    }

    pub fn pixels_per_unit(&self) -> f64 {
        self.pixels_per_unit
    }

    pub fn unit(&self) -> Unit {
        self.unit
    }

    /// Real length of `pixels` image pixels.
    pub fn length(&self, pixels: f64) -> f64 {
        pixels / self.pixels_per_unit
    }

    /// Real area of `square_pixels` image pixels squared.
    pub fn area(&self, square_pixels: f64) -> f64 {
        square_pixels / (self.pixels_per_unit * self.pixels_per_unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_pixel_length_over_entered_length() {
        let s = Scale::from_calibration(200.0, 2.0, Unit::Mm).unwrap();
        assert_eq!(s.pixels_per_unit(), 100.0);
        assert_eq!(s.unit(), Unit::Mm);
    }

    #[test]
    fn converts_length_and_area() {
        let s = Scale::from_calibration(200.0, 2.0, Unit::Mm).unwrap();
        assert_eq!(s.length(350.0), 3.5);
        assert_eq!(s.area(70_000.0), 7.0);
    }

    #[test]
    fn rejects_non_positive_and_non_finite_input() {
        for (p, l) in [
            (0.0, 1.0),
            (1.0, 0.0),
            (-5.0, 1.0),
            (5.0, -1.0),
            (f64::NAN, 1.0),
            (1.0, f64::NAN),
            (f64::INFINITY, 1.0),
            (1.0, f64::INFINITY),
        ] {
            assert_eq!(
                Scale::from_calibration(p, l, Unit::Mm),
                Err(ScaleError::NonPositive),
                "{p} / {l}"
            );
        }
    }

    #[test]
    fn rejects_pixel_unit() {
        assert_eq!(
            Scale::from_calibration(10.0, 1.0, Unit::Px),
            Err(ScaleError::NotPhysical)
        );
    }

    #[test]
    fn rejects_ratio_that_underflows_to_zero() {
        assert_eq!(
            Scale::from_calibration(f64::MIN_POSITIVE, f64::MAX, Unit::Um),
            Err(ScaleError::NonPositive)
        );
    }
}
