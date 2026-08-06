pub fn delta_counts(previous: &[u32], current: &[u32]) -> Vec<u32> {
    previous
        .iter()
        .zip(current.iter())
        .map(|(previous, current)| current.saturating_sub(*previous))
        .collect()
}

pub fn interval_row_counts(previous: &[u32], current: &[u32]) -> Vec<u32> {
    if row_total(current) >= row_total(previous) {
        delta_counts(previous, current)
    } else {
        current.to_vec()
    }
}

fn row_total(values: &[u32]) -> u64 {
    values.iter().map(|&value| value as u64).sum()
}

#[cfg(test)]
mod tests {
    use super::{delta_counts, interval_row_counts};

    #[test]
    fn delta_subtracts_previous() {
        assert_eq!(delta_counts(&[10, 20], &[15, 18]), vec![5, 0]);
    }

    #[test]
    fn delta_never_increases_on_drop() {
        assert_eq!(delta_counts(&[100, 200], &[90, 210]), vec![0, 10]);
    }

    #[test]
    fn interval_uses_delta_when_cumulative() {
        assert_eq!(interval_row_counts(&[10, 20], &[15, 25]), vec![5, 5]);
    }

    #[test]
    fn interval_uses_current_when_device_resets() {
        assert_eq!(interval_row_counts(&[1000, 2000], &[5, 8]), vec![5, 8]);
    }
}
