use std::collections::VecDeque;
use std::time::{Duration, Instant};

use radiacode_core::{AlarmLimits, AlarmLimitsUpdate, LiveRates};

const HISTORY_MINUTES: f64 = 10.0;
const MAX_SAMPLES: usize = 600;
const MIN_SAMPLE_SPACING: Duration = Duration::from_secs(1);
const OUTLIER_FACTOR: f32 = 3.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorSample {
    pub dose_rate: f32,
    pub count_rate: f32,
    pub elapsed: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorState {
    pub history: VecDeque<MonitorSample>,
    pub latest: Option<LiveRates>,
    pub limits: Option<AlarmLimits>,
    pub draft: AlarmLimitsDraft,
    pub started_at: Option<Instant>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlarmLimitsDraft {
    pub l1_dose_rate: f32,
    pub l2_dose_rate: f32,
    pub l1_count_rate: f32,
    pub l2_count_rate: f32,
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            latest: None,
            limits: None,
            draft: AlarmLimitsDraft {
                l1_dose_rate: 0.0,
                l2_dose_rate: 0.0,
                l1_count_rate: 0.0,
                l2_count_rate: 0.0,
            },
            started_at: None,
            status: "Connect a device to start monitoring.".into(),
        }
    }

    pub fn on_connect(&mut self) {
        self.history.clear();
        self.latest = None;
        self.started_at = Some(Instant::now());
        self.status = "Loading monitor data…".into();
    }

    pub fn on_disconnect(&mut self) {
        self.history.clear();
        self.latest = None;
        self.limits = None;
        self.started_at = None;
        self.status = "Connect a device to start monitoring.".into();
    }

    pub fn apply_limits(&mut self, limits: AlarmLimits) {
        self.draft = AlarmLimitsDraft {
            l1_dose_rate: limits.l1_dose_rate,
            l2_dose_rate: limits.l2_dose_rate,
            l1_count_rate: limits.l1_count_rate,
            l2_count_rate: limits.l2_count_rate,
        };
        self.limits = Some(limits);
    }

    pub fn push_sample(&mut self, rates: LiveRates) {
        let started_at = self.started_at.get_or_insert_with(Instant::now);
        let elapsed = started_at.elapsed();
        if self
            .history
            .back()
            .is_some_and(|sample| elapsed.saturating_sub(sample.elapsed) < MIN_SAMPLE_SPACING)
        {
            return;
        }
        let dose_rate = rates.dose_rate.max(0.0);
        let count_rate = rates.count_rate.max(0.0);
        if self.is_rate_outlier(dose_rate, count_rate) {
            return;
        }
        self.history.push_back(MonitorSample {
            dose_rate,
            count_rate,
            elapsed,
        });
        trim_history(&mut self.history, elapsed);
        self.latest = Some(LiveRates {
            dose_rate,
            count_rate,
            dose_unit_sv: rates.dose_unit_sv,
            count_unit_cpm: rates.count_unit_cpm,
        });
        self.status = "Live monitor".into();
    }

    pub fn limits_dirty(&self) -> bool {
        let Some(limits) = self.limits.as_ref() else {
            return false;
        };
        self.draft.l1_dose_rate != limits.l1_dose_rate
            || self.draft.l2_dose_rate != limits.l2_dose_rate
            || self.draft.l1_count_rate != limits.l1_count_rate
            || self.draft.l2_count_rate != limits.l2_count_rate
    }

    pub fn to_update(&self) -> AlarmLimitsUpdate {
        AlarmLimitsUpdate {
            l1_dose_rate: Some(self.draft.l1_dose_rate),
            l2_dose_rate: Some(self.draft.l2_dose_rate),
            l1_count_rate: Some(self.draft.l1_count_rate),
            l2_count_rate: Some(self.draft.l2_count_rate),
            dose_unit_sv: None,
            count_unit_cpm: None,
        }
    }

    pub fn dose_alarm_level(&self) -> AlarmLevel {
        alarm_level(
            self.latest.map(|sample| sample.dose_rate),
            self.limits.map(|limits| (limits.l1_dose_rate, limits.l2_dose_rate)),
        )
    }

    pub fn count_alarm_level(&self) -> AlarmLevel {
        alarm_level(
            self.latest.map(|sample| sample.count_rate),
            self.limits.map(|limits| (limits.l1_count_rate, limits.l2_count_rate)),
        )
    }

    fn is_rate_outlier(&self, dose_rate: f32, count_rate: f32) -> bool {
        if self.history.len() < 3 {
            return false;
        }
        let recent_dose: Vec<f32> = self
            .history
            .iter()
            .rev()
            .take(8)
            .map(|sample| sample.dose_rate)
            .collect();
        let recent_count: Vec<f32> = self
            .history
            .iter()
            .rev()
            .take(8)
            .map(|sample| sample.count_rate)
            .collect();
        dose_outlier(dose_rate, &recent_dose) || count_outlier(count_rate, &recent_count)
    }

    #[cfg(test)]
    pub fn default_for_tests() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmLevel {
    Normal,
    Warning,
    Danger,
}

fn trim_history(history: &mut VecDeque<MonitorSample>, elapsed: Duration) {
    let window = Duration::from_secs_f64(HISTORY_MINUTES * 60.0);
    while history.len() > MAX_SAMPLES {
        history.pop_front();
    }
    while history
        .front()
        .is_some_and(|sample| elapsed.saturating_sub(sample.elapsed) > window)
    {
        history.pop_front();
    }
}

fn alarm_level(value: Option<f32>, limits: Option<(f32, f32)>) -> AlarmLevel {
    let Some(value) = value else {
        return AlarmLevel::Normal;
    };
    let Some((l1, l2)) = limits else {
        return AlarmLevel::Normal;
    };
    if value >= l2 {
        AlarmLevel::Danger
    } else if value >= l1 {
        AlarmLevel::Warning
    } else {
        AlarmLevel::Normal
    }
}

fn dose_outlier(value: f32, recent: &[f32]) -> bool {
    rate_outlier(value, recent)
}

fn count_outlier(value: f32, recent: &[f32]) -> bool {
    rate_outlier(value, recent)
}

fn rate_outlier(value: f32, recent: &[f32]) -> bool {
    if recent.len() < 3 {
        return false;
    }
    let mut sorted = recent.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2].max(0.001);
    value > median * OUTLIER_FACTOR || value < median / OUTLIER_FACTOR
}
