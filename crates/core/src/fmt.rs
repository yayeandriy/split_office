//! Canonical formatting utilities for the Split Office domain.
//!
//! All human-readable number formatting must go through these functions.
//! Do not duplicate formatting logic in UI, profiler, or grid crates.

/// Format a large integer with a human-readable suffix (k / M).
///
/// # Examples
/// ```
/// use core::fmt_large;
/// assert_eq!(fmt_large(999),          "999");
/// assert_eq!(fmt_large(1_500),        "1.5k");
/// assert_eq!(fmt_large(2_000_000),    "2.0M");
/// ```
pub fn fmt_large(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_large_small() {
        assert_eq!(fmt_large(0), "0");
        assert_eq!(fmt_large(999), "999");
    }

    #[test]
    fn test_fmt_large_thousands() {
        assert_eq!(fmt_large(1_000), "1.0k");
        assert_eq!(fmt_large(1_500), "1.5k");
        assert_eq!(fmt_large(999_999), "1000.0k");
    }

    #[test]
    fn test_fmt_large_millions() {
        assert_eq!(fmt_large(1_000_000), "1.0M");
        assert_eq!(fmt_large(2_500_000), "2.5M");
    }
}
