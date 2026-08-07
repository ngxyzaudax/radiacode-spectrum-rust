use std::collections::VecDeque;

use radiacode_core::{AccumulatedDose, AlarmLimits};

use crate::monitor::AlarmLevel;

const MAX_SAMPLES: usize = 600;
const MIN_SPACING_SECS: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoseHistoryPoint {
    pub duration_secs: u32,
    pub dose: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DosimeterState {
    pub history: VecDeque<DoseHistoryPoint>,
    pub latest: Option<AccumulatedDose>,
    pub limits: Option<AlarmLimits>,
    pub status: String,
}

impl DosimeterState {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            latest: None,
            limits: None,
            status: "Connect a device to view accumulated dose.".into(),
        }
    }

    pub fn on_connect(&mut self) {
        self.clear_session();
        self.status = "Loading dosimeter data…".into();
    }

    pub fn on_disconnect(&mut self) {
        self.clear_session();
        self.status = "Connect a device to view accumulated dose.".into();
    }

    pub fn on_reset(&mut self) {
        self.history.clear();
        self.latest = None;
        self.status = "Dose reset. Waiting for data…".into();
    }

    pub fn apply_limits(&mut self, limits: AlarmLimits) {
        self.limits = Some(limits);
    }

    pub fn push_sample(&mut self, sample: AccumulatedDose) {
        if self.history.back().is_some_and(|point| {
            sample
                .duration_secs
                .saturating_sub(point.duration_secs)
                < MIN_SPACING_SECS
        }) {
            return;
        }
        let dose = sample.dose.max(0.0);
        self.history.push_back(DoseHistoryPoint {
            duration_secs: sample.duration_secs,
            dose,
        });
        while self.history.len() > MAX_SAMPLES {
            self.history.pop_front();
        }
        self.latest = Some(AccumulatedDose {
            dose,
            duration_secs: sample.duration_secs,
            dose_unit_sv: sample.dose_unit_sv,
        });
        self.status = "Live dosimeter".into();
    }

    pub fn dose_alarm_level(&self) -> AlarmLevel {
        alarm_level(
            self.latest.map(|sample| sample.dose),
            self.limits.map(|limits| (limits.l1_dose, limits.l2_dose)),
        )
    }

    fn clear_session(&mut self) {
        self.history.clear();
        self.latest = None;
        self.limits = None;
    }
}

pub fn format_session_duration(secs: u32) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
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
