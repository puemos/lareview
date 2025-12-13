pub(crate) fn clamp_width(value: f32, min: f32, max: f32) -> f32 {
    let value = if value.is_finite() { value } else { 0.0 };
    let mut min = if min.is_finite() { min } else { 0.0 };
    let mut max = if max.is_finite() { max } else { min };

    if min < 0.0 {
        min = 0.0;
    }
    if max < 0.0 {
        max = 0.0;
    }
    if max < min {
        min = max;
    }

    value.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_width_handles_max_less_than_min() {
        let v = clamp_width(300.0, 250.0, 243.5);
        assert!((v - 243.5).abs() < f32::EPSILON);
    }

    #[test]
    fn clamp_width_handles_nan_inputs() {
        let v = clamp_width(f32::NAN, 250.0, 243.5);
        assert!(v.is_finite());
    }
}
