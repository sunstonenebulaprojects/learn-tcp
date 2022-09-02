use std::cmp::Ordering;

/// Taken from RFC 1982 Serial Number Arithmetic
///
/// Less:
/// (i1 < i2 and i2 - i1 < 2^(SERIAL_BITS - 1)) or
/// (i1 > i2 and i1 - i2 > 2^(SERIAL_BITS - 1))
///
/// Greater:
/// (i1 < i2 and i2 - i1 > 2^(SERIAL_BITS - 1)) or
/// (i1 > i2 and i1 - i2 < 2^(SERIAL_BITS - 1))
///
/// My reading for these comparisons:
/// first we check obvious case (i1 < i2 for Less and i1 > i2 for Greater)
/// then we check if there was a wrapping op
/// for Less:
/// i1 + n >= 2^SERIAL_BITS and therefor i1 > i2
///     i1 - i2 > 2^(SERIAL_BITS - 1)) also means that i2.wrapping_sub(i1) < 2^(SERIAL_BITS - 1)
/// for Greater:
/// i2 + n >= 2^SERIAL_BITS and therefor i1 < i2
///     i2 - i1 > 2^(SERIAL_BITS - 1)) also means that i1.wrapping_sub(i2) < 2^(SERIAL_BITS - 1)
/// So we simply check the difference to be < 2^(SERIAL_BITS - 1) for normal and wrapping cases
pub fn sn_cmp(left: u32, right: u32) -> Ordering {
    if left < right && (right.wrapping_sub(left)) < 2u32.pow(32 - 1)
        || left > right && (left.wrapping_sub(right)) > 2u32.pow(32 - 1)
    {
        Ordering::Less
    } else if left < right && right.wrapping_sub(left) > 2u32.pow(32 - 1)
        || left > right && left.wrapping_sub(right) < 2u32.pow(32 - 1)
    {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn less_test() {
        // (i1 < i2 and i2 - i1 < 2^(SERIAL_BITS - 1))
        assert_eq!(sn_cmp(2, 5), Ordering::Less);
        // (i1 > i2 and i1 - i2 > 2^(SERIAL_BITS - 1))
        assert_eq!(sn_cmp(2u32.pow(31) + 101, 100), Ordering::Less);
    }

    #[test]
    fn greater_test() {
        // (i1 < i2 and i2 - i1 > 2^(SERIAL_BITS - 1))
        assert_eq!(sn_cmp(20, 5), Ordering::Greater);
        // (i1 > i2 and i1 - i2 < 2^(SERIAL_BITS - 1))
        assert_eq!(sn_cmp(0, (2u64.pow(32) - 1u64) as u32), Ordering::Greater);
    }

    #[test]
    fn equal_test() {
        assert_eq!(sn_cmp(20, 20), Ordering::Equal);
    }
}
