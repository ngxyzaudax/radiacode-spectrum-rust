pub fn moving_average(counts: &[u32], window: usize) -> Vec<f64> {
    if window <= 1 {
        return counts.iter().map(|&count| count as f64).collect();
    }
    let half = window / 2;
    let length = counts.len();
    (0..length)
        .map(|index| {
            let start = index.saturating_sub(half);
            let end = (index + half + 1).min(length);
            let sum: u64 = counts[start..end].iter().map(|&count| count as u64).sum();
            sum as f64 / (end - start) as f64
        })
        .collect()
}

pub fn normalize_window(value: usize) -> usize {
    value.clamp(1, 16)
}
