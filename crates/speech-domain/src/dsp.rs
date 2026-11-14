const FRACTION_BITS: u32 = 16;
const ONE: u64 = 1 << FRACTION_BITS;
const FRACTION_MASK: u64 = ONE - 1;
const FRACTION_SCALE: f32 = 65_536.0;

/// Reads `samples` at positions `0, step, 2·step, …` (fixed point, 16 fractional bits) with
/// linear interpolation. `step` above 1.0 shortens the signal, below 1.0 lengthens it.
pub(crate) fn stretch(samples: &[f32], step: u64) -> Vec<f32> {
    if step == 0 {
        return Vec::new();
    }
    let input_len = u64::try_from(samples.len()).unwrap_or(u64::MAX);
    let output_len = input_len.saturating_mul(ONE) / step;
    (0..output_len)
        .map(|index| sample_at(samples, index.saturating_mul(step)))
        .collect()
}

/// The fixed-point step that turns `numerator` units into `denominator` units.
pub(crate) fn step(numerator: u32, denominator: u32) -> u64 {
    u64::from(numerator).saturating_mul(ONE) / u64::from(denominator.max(1))
}

fn sample_at(samples: &[f32], position: u64) -> f32 {
    let index = usize::try_from(position >> FRACTION_BITS).unwrap_or(usize::MAX);
    let fraction = u16::try_from(position & FRACTION_MASK).map_or(0.0, f32::from) / FRACTION_SCALE;
    let current = samples.get(index).copied().unwrap_or(0.0);
    let next = samples
        .get(index.saturating_add(1))
        .copied()
        .unwrap_or(current);
    (next - current).mul_add(fraction, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_step_copies_the_input() {
        let input = [0.0, 0.5, -0.5, 1.0];
        assert_eq!(stretch(&input, ONE), input);
    }

    #[test]
    fn half_step_doubles_the_length_and_interpolates() {
        assert_eq!(stretch(&[0.0, 1.0], ONE / 2), [0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn double_step_halves_the_length() {
        assert_eq!(stretch(&[0.0, 0.1, 0.2, 0.3], ONE * 2), [0.0, 0.2]);
    }

    #[test]
    fn empty_input_and_zero_step_give_nothing() {
        assert!(stretch(&[], ONE).is_empty(), "no samples in, none out");
        assert!(stretch(&[1.0], 0).is_empty(), "zero step is rejected");
    }

    #[test]
    fn step_is_a_fixed_point_ratio() {
        assert_eq!(step(1, 1), ONE);
        assert_eq!(step(22_050, 44_100), ONE / 2);
        assert_eq!(step(3, 0), 3 * ONE);
    }
}
