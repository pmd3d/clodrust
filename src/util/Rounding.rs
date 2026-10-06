//! Integer alignment helpers backed by the verified checked arithmetic kernel.
use super::CheckedLayout;

pub fn round_away_from_zero(alignment: i64, value: i64) -> i64 {
    CheckedLayout::round_away(alignment, value)
        .unwrap_or_else(|| panic!("Invalid alignment or layout rounding overflow"))
}

pub fn round_up(value: usize, alignment: usize) -> usize {
    CheckedLayout::round_up(value, alignment)
        .unwrap_or_else(|| panic!("Invalid alignment or layout rounding overflow"))
}
