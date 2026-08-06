use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};
use radiacode_core::{
    AlarmLimits, AlarmLimitsUpdate, DeviceEndpoint, DeviceStatus, DiscoveredDevice, LiveRates,
    RadiaCode, SessionRestore,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, info};

use crate::model::{DeviceInfo, SpectrumView};
use crate::spectrogram::capture::{spawn_capture_router, SpectrogramCapture};
use crate::worker_ops::{
    handle_connect, handle_disconnect, handle_monitor, handle_reset, handle_scan,
    handle_set_alarm_limits, handle_spectrum, SessionEpoch,
};

#[derive(Debug, Clone)]
pub enum WorkerCommand {
    Scan,
    Connect {
        endpoint: DeviceEndpoint,
        hint_rssi: Option<i16>,
    },
    Disconnect,
    FetchSpectrum,
    ResetSpectrum,
    FetchMonitor,
    SetAlarmLimits(AlarmLimitsUpdate),
    SetCaptureInterval(f64),
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    ScanFinished(Vec<DiscoveredDevice>),
    Connected(DeviceInfo),
    Disconnected,
    Reconnecting,
    UsbPermissionRequired { endpoint: DeviceEndpoint },
    Spectrum(SpectrumView),
    DeviceStatus(DeviceStatus),
    MonitorSample(LiveRates),
    MonitorPollComplete,
    AlarmLimits(AlarmLimits),
    Error(String),
    Busy(bool),
}

pub struct WorkerHandle {
    pub commands: UnboundedSender<WorkerCommand>,
    pub events: Receiver<WorkerEvent>,
    session_epoch: Arc<AtomicU64>,
}

impl WorkerHandle {
    pub fn end_session(&self) {
        self.session_epoch.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn spawn_worker(capture: Arc<Mutex<SpectrogramCapture>>) -> WorkerHandle {
    let (commands, command_rx) = unbounded_channel();
    let (worker_event_tx, worker_event_rx) = unbounded();
    let (ui_event_tx, ui_event_rx) = unbounded();
    spawn_capture_router(worker_event_rx, ui_event_tx, capture);
    let session_epoch = Arc::new(AtomicU64::new(0));
    let worker_epoch = Arc::clone(&session_epoch);
    info!("spawning device worker thread");
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        runtime.block_on(worker_loop(command_rx, worker_event_tx, worker_epoch));
    });
    WorkerHandle {
        commands,
        events: ui_event_rx,
        session_epoch,
    }
}

const MONITOR_POLL_SECS: u64 = 1;

struct CoalescedBatch {
    priority: Option<WorkerCommand>,
    fetch_monitor: bool,
    fetch_spectrum: bool,
}

impl CoalescedBatch {
    fn absorb(&mut self, command: WorkerCommand) {
        match command {
            WorkerCommand::FetchMonitor => self.fetch_monitor = true,
            WorkerCommand::FetchSpectrum => self.fetch_spectrum = true,
            priority => self.priority = Some(priority),
        }
    }
}

async fn recv_batch(commands: &mut UnboundedReceiver<WorkerCommand>) -> Option<CoalescedBatch> {
    let first = commands.recv().await?;
    let mut batch = CoalescedBatch {
        priority: None,
        fetch_monitor: false,
        fetch_spectrum: false,
    };
    batch.absorb(first);
    while batch.priority.is_none() {
        match commands.try_recv() {
            Ok(next) => batch.absorb(next),
            Err(_) => break,
        }
    }
    Some(batch)
}

fn capture_duration(secs: f64) -> Duration {
    Duration::from_secs_f64(secs.clamp(1.0, 600.0))
}

fn reset_session(
    device: &mut Option<RadiaCode>,
    session_endpoint: &mut Option<DeviceEndpoint>,
    alarm_limits: &mut Option<AlarmLimits>,
    monitor_polls: &mut u64,
    link_status: &mut DeviceStatus,
    session_restore: &mut Option<SessionRestore>,
) {
    *session_endpoint = None;
    *alarm_limits = None;
    *monitor_polls = 0;
    *link_status = DeviceStatus::default();
    *session_restore = None;
    *device = None;
}

async fn run_fetch_batch(
    batch: CoalescedBatch,
    events: &Sender<WorkerEvent>,
    session_epoch: &Arc<AtomicU64>,
    device: &mut Option<RadiaCode>,
    session_endpoint: &mut Option<DeviceEndpoint>,
    alarm_limits: &mut Option<AlarmLimits>,
    monitor_polls: &mut u64,
    link_status: &mut DeviceStatus,
    session_restore: &mut Option<SessionRestore>,
) {
    if batch.fetch_monitor || batch.fetch_spectrum {
        debug!(
            fetch_monitor = batch.fetch_monitor,
            fetch_spectrum = batch.fetch_spectrum,
            session_endpoint = ?session_endpoint,
            "worker coalesced fetch batch"
        );
    }
    let _ = events.send(WorkerEvent::Busy(true));
    let session = SessionEpoch {
        live: Arc::clone(session_epoch),
        started: session_epoch.load(Ordering::SeqCst),
    };
    if batch.fetch_monitor {
        *device = handle_monitor(
            events,
            device.take(),
            session_endpoint.as_ref(),
            alarm_limits,
            monitor_polls,
            &session,
            link_status,
            session_restore,
        )
        .await;
        if device.is_none() {
            reset_session(
                device,
                session_endpoint,
                alarm_limits,
                monitor_polls,
                link_status,
                session_restore,
            );
        }
    }
    if batch.fetch_spectrum && device.is_some() {
        *device = handle_spectrum(
            events,
            device.take(),
            session_endpoint.as_ref(),
            &session,
            link_status,
            session_restore,
        )
        .await;
        if device.is_none() {
            reset_session(
                device,
                session_endpoint,
                alarm_limits,
                monitor_polls,
                link_status,
                session_restore,
            );
        }
    }
    let _ = events.send(WorkerEvent::Busy(false));
}

async fn worker_loop(
    mut commands: UnboundedReceiver<WorkerCommand>,
    events: Sender<WorkerEvent>,
    session_epoch: Arc<AtomicU64>,
) {
    let mut device: Option<RadiaCode> = None;
    let mut session_endpoint: Option<DeviceEndpoint> = None;
    let mut alarm_limits: Option<AlarmLimits> = None;
    let mut monitor_polls: u64 = 0;
    let mut link_status = DeviceStatus::default();
    let mut session_restore: Option<SessionRestore> = None;
    let mut capture_interval_secs = 5.0;
    let mut spectrum_tick = time::interval(capture_duration(capture_interval_secs));
    spectrum_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut monitor_tick = time::interval(Duration::from_secs(MONITOR_POLL_SECS));
    monitor_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    debug!("device worker loop ready");
    loop {
        tokio::select! {
            batch = recv_batch(&mut commands) => {
                let Some(batch) = batch else {
                    break;
                };
                if let Some(command) = batch.priority {
                    if let WorkerCommand::SetCaptureInterval(secs) = command {
                        capture_interval_secs = secs;
                        spectrum_tick = time::interval(capture_duration(capture_interval_secs));
                        spectrum_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
                        debug!(capture_interval_secs, "worker capture interval updated");
                        continue;
                    }
                    debug!(?command, session_endpoint = ?session_endpoint, "worker command");
                    run_command(
                        command,
                        &events,
                        &session_epoch,
                        &mut device,
                        &mut session_endpoint,
                        &mut alarm_limits,
                        &mut monitor_polls,
                        &mut link_status,
                        &mut session_restore,
                    )
                    .await;
                    if device.is_some() {
                        spectrum_tick.reset();
                        monitor_tick.reset();
                    }
                    continue;
                }
                run_fetch_batch(
                    batch,
                    &events,
                    &session_epoch,
                    &mut device,
                    &mut session_endpoint,
                    &mut alarm_limits,
                    &mut monitor_polls,
                    &mut link_status,
                    &mut session_restore,
                )
                .await;
            }
            _ = spectrum_tick.tick(), if device.is_some() => {
                debug!(capture_interval_secs, "worker background spectrum fetch");
                run_fetch_batch(
                    CoalescedBatch {
                        priority: None,
                        fetch_monitor: false,
                        fetch_spectrum: true,
                    },
                    &events,
                    &session_epoch,
                    &mut device,
                    &mut session_endpoint,
                    &mut alarm_limits,
                    &mut monitor_polls,
                    &mut link_status,
                    &mut session_restore,
                )
                .await;
            }
            _ = monitor_tick.tick(), if device.is_some() => {
                debug!("worker background monitor fetch");
                run_fetch_batch(
                    CoalescedBatch {
                        priority: None,
                        fetch_monitor: true,
                        fetch_spectrum: false,
                    },
                    &events,
                    &session_epoch,
                    &mut device,
                    &mut session_endpoint,
                    &mut alarm_limits,
                    &mut monitor_polls,
                    &mut link_status,
                    &mut session_restore,
                )
                .await;
            }
        }
    }
    info!("device worker loop ended");
}

async fn run_command(
    command: WorkerCommand,
    events: &Sender<WorkerEvent>,
    session_epoch: &Arc<AtomicU64>,
    device: &mut Option<RadiaCode>,
    session_endpoint: &mut Option<DeviceEndpoint>,
    alarm_limits: &mut Option<AlarmLimits>,
    monitor_polls: &mut u64,
    link_status: &mut DeviceStatus,
    session_restore: &mut Option<SessionRestore>,
) {
    let _ = events.send(WorkerEvent::Busy(true));
    let session = SessionEpoch {
        live: Arc::clone(session_epoch),
        started: session_epoch.load(Ordering::SeqCst),
    };
    match command {
        WorkerCommand::Scan => handle_scan(events).await,
        WorkerCommand::Connect { endpoint, hint_rssi } => {
            *alarm_limits = None;
            *monitor_polls = 0;
            *link_status = DeviceStatus::default();
            *session_restore = None;
            *device = handle_connect(
                events,
                device.take(),
                &endpoint,
                hint_rssi,
                &session,
                link_status,
                session_restore,
            )
            .await;
            *session_endpoint = device.as_ref().map(|_| endpoint);
        }
        WorkerCommand::Disconnect => {
            session_epoch.fetch_add(1, Ordering::SeqCst);
            *session_endpoint = None;
            *alarm_limits = None;
            *monitor_polls = 0;
            *link_status = DeviceStatus::default();
            *session_restore = None;
            handle_disconnect(events, device.take()).await;
        }
        WorkerCommand::FetchSpectrum => {
            *device = handle_spectrum(
                events,
                device.take(),
                session_endpoint.as_ref(),
                &session,
                link_status,
                session_restore,
            )
            .await;
            if device.is_none() {
                *session_endpoint = None;
                *alarm_limits = None;
                *monitor_polls = 0;
                *link_status = DeviceStatus::default();
                *session_restore = None;
            }
        }
        WorkerCommand::ResetSpectrum => {
            *device = handle_reset(
                events,
                device.take(),
                session_endpoint.as_ref(),
                &session,
                link_status,
                session_restore,
            )
            .await;
            if device.is_none() {
                *session_endpoint = None;
                *alarm_limits = None;
                *monitor_polls = 0;
                *link_status = DeviceStatus::default();
                *session_restore = None;
            }
        }
        WorkerCommand::FetchMonitor => {
            *device = handle_monitor(
                events,
                device.take(),
                session_endpoint.as_ref(),
                alarm_limits,
                monitor_polls,
                &session,
                link_status,
                session_restore,
            )
            .await;
            if device.is_none() {
                *session_endpoint = None;
                *alarm_limits = None;
                *monitor_polls = 0;
                *session_restore = None;
            }
        }
        WorkerCommand::SetAlarmLimits(update) => {
            *device = handle_set_alarm_limits(
                events,
                device.take(),
                session_endpoint.as_ref(),
                update,
                alarm_limits,
                &session,
                link_status,
                session_restore,
            )
            .await;
            if device.is_none() {
                *session_endpoint = None;
                *alarm_limits = None;
                *monitor_polls = 0;
                *link_status = DeviceStatus::default();
                *session_restore = None;
            }
        }
        WorkerCommand::SetCaptureInterval(_) => {}
    }
    let _ = events.send(WorkerEvent::Busy(false));
}
