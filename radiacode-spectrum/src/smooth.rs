pub fn moving_average(counts: &[u32], window: usize) -> Vec<f64> {
    let values: Vec<f64> = counts.iter().map(|&count| count as f64).collect();
    moving_average_f64(&values, window)
}

pub fn moving_average_f64(values: &[f64], window: usize) -> Vec<f64> {
    if window <= 1 {
        return values.to_vec();
    }
    let half = window / 2;
    let length = values.len();
    (0..length)
        .map(|index| {
            let start = index.saturating_sub(half);
            let end = (index + half + 1).min(length);
            let sum: f64 = values[start..end].iter().sum();
            sum / (end - start) as f64
        })
        .collect()
}

pub fn normalize_window(value: usize) -> usize {
    value.clamp(1, 16)
}
