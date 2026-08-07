#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use crate::model::SpectrumView;
    use crate::spectrogram::capture::SpectrogramCapture;
    use crate::spectrogram::model::RowKind;
    use crate::spectrogram::state::SpectrogramState;

    fn test_state() -> SpectrogramState {
        let capture = Arc::new(Mutex::new(SpectrogramCapture::new()));
        let mut state = SpectrogramState::new(capture);
        state.settings.capture_interval_secs = 5.0;
        if let Ok(mut cap) = state.capture.lock() {
            cap.settings.capture_interval_secs = 5.0;
            cap.on_session_connect("test");
        }
        state.sync_from_capture();
        state
    }

    fn sample_spectrum(total: u32, duration_secs: u64) -> SpectrumView {
        SpectrumView {
            duration: Duration::from_secs(duration_secs),
            a0: 0.0,
            a1: 1.0,
            a2: 0.0,
            counts: vec![total; 512],
            total_counts: total as u64 * 512,
        }
    }

    #[test]
    fn reconnect_baseline_then_normal_row() {
        let mut state = test_state();
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1);
        state.ingest_spectrum(&sample_spectrum(20, 10), None, 2);
        assert_eq!(state.live_row_count(), 1);

        state.on_reconnect();
        state.ingest_spectrum(&sample_spectrum(5000, 60), None, 3);
        assert_eq!(state.live_row_count(), 1);
        state.ingest_spectrum(&sample_spectrum(5010, 65), None, 4);
        assert_eq!(state.live_row_count(), 2);
        assert!(matches!(
            state.live_series.as_ref().unwrap().rows[1].kind,
            RowKind::Normal
        ));
    }

    #[test]
    fn duration_regression_rebaselines_instead_of_freezing() {
        let mut state = test_state();
        state.settings.capture_interval_secs = 10.0;
        if let Ok(mut cap) = state.capture.lock() {
            cap.settings.capture_interval_secs = 10.0;
        }
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1);
        state.ingest_spectrum(&sample_spectrum(20, 15), None, 2);
        assert_eq!(state.live_row_count(), 1);
        state.ingest_spectrum(&sample_spectrum(30, 17240), None, 3);
        assert_eq!(state.live_row_count(), 2);
        state.ingest_spectrum(&sample_spectrum(40, 1001), None, 4);
        assert_eq!(state.live_row_count(), 2);
        state.ingest_spectrum(&sample_spectrum(50, 1011), None, 5);
        assert_eq!(state.live_row_count(), 3);
    }

    #[test]
    fn short_interval_skips_row_append() {
        let mut state = test_state();
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1);
        state.ingest_spectrum(&sample_spectrum(20, 10), None, 2);
        assert_eq!(state.live_row_count(), 1);
        state.ingest_spectrum(&sample_spectrum(25, 12), None, 3);
        assert_eq!(state.live_row_count(), 1);
        state.ingest_spectrum(&sample_spectrum(35, 20), None, 4);
        assert_eq!(state.live_row_count(), 2);
    }

    #[test]
    fn long_gap_produces_gap_recovery_row() {
        let mut state = test_state();
        state.ingest_spectrum(&sample_spectrum(10, 5), None, 1);
        state.ingest_spectrum(&sample_spectrum(20, 10), None, 2);
        state.ingest_spectrum(&sample_spectrum(2000, 55), None, 3);
        assert_eq!(state.live_row_count(), 2);
        assert!(matches!(
            state.live_series.as_ref().unwrap().rows[1].kind,
            RowKind::GapRecovery { .. }
        ));
    }
}
