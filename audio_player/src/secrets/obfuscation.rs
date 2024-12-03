//! Simple obfuscation algorithm for secret data.

/// Returns the smallest x for which the x-th triangular number is smaller than or equal to `max`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn triangular_numbers_count(max: usize) -> usize {
    // (n^2 + n) / 2 = max
    // n^2 + n - 2 * max = 0
    // Δ = b^2 - 4ac = 1^2 - 4 * 1 * (-2 * max) = 1 + 8 * max
    // -> n1 = (-b - sqrt(Δ)) / 2a (will be smaller than 0, not intereting)
    // -> n2 = (-b + sqrt(Δ)) / 2a = (-1 + sqrt(1 + 8 * max)) / 2
    // We round up to include the last triangular number
    ((-1.0 + (1.0 + 8.0 * (max - 1) as f64).sqrt()) / 2.0).ceil() as usize
}

/// Returns an iterator over the triangular numbers from 1 to at `max`.
///
/// It includes the next triangular number if `max` is not a triangular number.
fn triangular_numbers(max: usize) -> impl DoubleEndedIterator<Item = usize> + Sized {
    (1..=triangular_numbers_count(max)).filter(move |&x| x < max).map(|x| (x * x + x) / 2)
}

/// Obfuscates some `data`: returns a list of `(normal_pos, obfuscated_pos)`.
fn two_way_obfuscate(length: usize) -> std::vec::Vec<(usize, usize)> {
    let mut ret = vec![];
    // The index of the clear data
    let mut index = 0;
    // The maximum number of passes in an iteration
    let max_passes = triangular_numbers_count(length + 1);
    for step_num in 0..=max_passes {
        let iter = triangular_numbers(length + 1);
        let reverse = step_num % 2 == 0;
        let mut to_skip = step_num;
        let mut iterated = 0;
        let op = |n| {
            if reverse {
                if iterated + to_skip >= max_passes {
                    return;
                }
                iterated += 1;
            } else if to_skip > 0 {
                to_skip -= 1;
                return;
            }
            let n = n - step_num - 1;
            if n < length {
                ret.push((n, index));
                index += 1;
            }
        };
        if reverse {
            iter.rev().for_each(op);
        } else {
            iter.for_each(op);
        }
    }
    ret
}

/// Obfuscate some `data`.
pub fn obfuscate<T: Clone + Default>(data: &[T]) -> Vec<T> {
    let mut ret = vec![Default::default(); data.len()];
    for (orig_pos, obfuscated_pos) in two_way_obfuscate(data.len()) {
        ret[obfuscated_pos] = data[orig_pos].clone();
    }
    ret
}

/// Deobfuscate some `data`.
pub fn deobfuscate<T: Clone + Default>(data: &[T]) -> Vec<T> {
    let mut ret = vec![Default::default(); data.len()];
    for (orig_pos, obfuscated_pos) in two_way_obfuscate(data.len()) {
        ret[orig_pos] = data[obfuscated_pos].clone();
    }
    ret
}

#[cfg(test)]
mod tests {
    use tinyrand::Seeded;

    use super::{deobfuscate, obfuscate, triangular_numbers};

    /// Checks that the triangular numbers are in the asked bounds.
    #[test]
    fn test_triangular_numbers() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::float_cmp)]
        fn is_triangular_number(n: usize) -> bool {
            let result = (8.0 * n as f64 + 1.0).sqrt();
            result as usize as f64 == result
        }

        for i in 1..=1000 {
            let mut last = false;
            for n in triangular_numbers(i) {
                if n > i && !is_triangular_number(i) && !last {
                    // If there is a number outside the interval and the maximum value
                    // is not a triangular number, allow it, but not the next number
                    last = true;
                } else {
                    assert!(n <= i);
                }
            }
        }
    }

    /// Checks that the data doesn't change after an obfuscation and a deobfuscation.
    #[test]
    fn two_way() {
        let string: Vec<u8> = "abcdefghijklmnopqrstuvwxyz".into();
        let obfuscated = obfuscate(&string);
        assert_eq!(string.len(), obfuscated.len());
        assert_ne!(string, obfuscated);
        let deobfuscated = deobfuscate(&obfuscated);
        assert_eq!(string, deobfuscated);
    }

    /// Checks that the data is conserved during the obfuscation.
    #[test]
    fn integrity() {
        let chars: Vec<u8> = (97..123).collect(); // a..z
        let mut result_chars = obfuscate(&chars[..]);
        assert_eq!(result_chars.len(), 26);
        assert!(!result_chars.is_sorted());
        result_chars.sort_unstable();
        assert_eq!(result_chars, chars);
    }
}
