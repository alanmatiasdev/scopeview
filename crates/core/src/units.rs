//! Measurement units.

/// A unit a measurement can be displayed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    /// Image pixels, used while no calibration is active.
    Px,
    Mm,
    Um,
}

impl Unit {
    /// Text appended to a displayed value.
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Px => "px",
            Unit::Mm => "mm",
            Unit::Um => "µm",
        }
    }

    /// True for units a calibration can use.
    pub fn is_physical(self) -> bool {
        !matches!(self, Unit::Px)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffixes() {
        assert_eq!(Unit::Px.suffix(), "px");
        assert_eq!(Unit::Mm.suffix(), "mm");
        assert_eq!(Unit::Um.suffix(), "µm");
    }

    #[test]
    fn only_mm_and_um_are_physical() {
        assert!(Unit::Mm.is_physical());
        assert!(Unit::Um.is_physical());
        assert!(!Unit::Px.is_physical());
    }
}
