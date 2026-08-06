use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use radiacode_bluetooth::{
    AlarmLimits, AlarmLimitsUpdate, DeviceStatus, LiveRates, RadiaCode, ScannedDevice,
    SessionRestore,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{debug, info};

use crate::model::{DeviceInfo, SpectrumView};
use crate::worker_ops::{
    handle_connect, handle_disconnect, handle_monitor, handle_reset, handle_scan,
    handle_set_alarm_limits, handle_spectrum, SessionEpoch,
};

#[derive(Debug)]
pub enum WorkerCommand {
    Scan,
    Connect(String),
    Disconnect,
    FetchSpectrum,
    ResetSpectrum,
    FetchMonitor,
    SetAlarmLimits(AlarmLimitsUpdate),
}

#[derive(Debug)]
pub enum WorkerEvent {
    ScanFinished(Vec<ScannedDevice>),
    Connected(DeviceInfo),
    Disconnected,
    Reconnecting,
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

pub fn spawn_worker() -> WorkerHandle {
    let (commands, command_rx) = unbounded_channel();
    let (event_tx, events) = mpsc::channel();
    let session_epoch = Arc::new(AtomicU64::new(0));
    let worker_epoch = Arc::clone(&session_epoch);
    info!("spawning bluetooth worker thread");
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        runtime.block_on(worker_loop(command_rx, event_tx, worker_epoch));
    });
    WorkerHandle {
        commands,
        events,
        session_epoch,
    }
}

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

async fn worker_loop(
    mut commands: UnboundedReceiver<WorkerCommand>,
    events: Sender<WorkerEvent>,
    session_epoch: Arc<AtomicU64>,
) {
    let mut device: Option<RadiaCode> = None;
    let mut session_mac: Option<String> = None;
    let mut alarm_limits: Option<AlarmLimits> = None;
    let mut monitor_polls: u64 = 0;
    let mut link_status = DeviceStatus::default();
    let mut session_restore: Option<SessionRestore> = None;
    debug!("bluetooth worker loop ready");
    while let Some(batch) = recv_batch(&mut commands).await {
        if let Some(command) = batch.priority {
            debug!(?command, session_mac = session_mac.as_deref(), "worker command");
            run_command(
                command,
                &events,
                &session_epoch,
                &mut device,
                &mut session_mac,
                &mut alarm_limits,
                &mut monitor_polls,
                &mut link_status,
                &mut session_restore,
            )
            .await;
            continue;
        }
        if batch.fetch_monitor || batch.fetch_spectrum {
            debug!(
                fetch_monitor = batch.fetch_monitor,
                fetch_spectrum = batch.fetch_spectrum,
                session_mac = session_mac.as_deref(),
                "worker coalesced fetch batch"
            );
        }
        let _ = events.send(WorkerEvent::Busy(true));
        let session = SessionEpoch {
            live: Arc::clone(&session_epoch),
            started: session_epoch.load(Ordering::SeqCst),
        };
        if batch.fetch_monitor {
            device = handle_monitor(
                &events,
                device.take(),
                session_mac.as_deref(),
                &mut alarm_limits,
                &mut monitor_polls,
                &session,
                &mut link_status,
                &session_restore,
            )
            .await;
            if device.is_none() {
                session_mac = None;
                alarm_limits = None;
                monitor_polls = 0;
                link_status = DeviceStatus::default();
                session_restore = None;
            }
        }
        if batch.fetch_spectrum && device.is_some() {
            device = handle_spectrum(
                &events,
                device.take(),
                session_mac.as_deref(),
                &session,
                &mut link_status,
                &session_restore,
            )
            .await;
            if device.is_none() {
                session_mac = None;
                alarm_limits = None;
                monitor_polls = 0;
                link_status = DeviceStatus::default();
                session_restore = None;
            }
        }
        let _ = events.send(WorkerEvent::Busy(false));
    }
    info!("bluetooth worker loop ended");
}

async fn run_command(
    command: WorkerCommand,
    events: &Sender<WorkerEvent>,
    session_epoch: &Arc<AtomicU64>,
    device: &mut Option<RadiaCode>,
    session_mac: &mut Option<String>,
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
        WorkerCommand::Connect(mac) => {
            *alarm_limits = None;
            *monitor_polls = 0;
            *link_status = DeviceStatus::default();
            *session_restore = None;
            *device = handle_connect(
                events,
                device.take(),
                &mac,
                &session,
                link_status,
                session_restore,
            )
            .await;
            *session_mac = device.as_ref().map(|_| mac);
        }
        WorkerCommand::Disconnect => {
            session_epoch.fetch_add(1, Ordering::SeqCst);
            *session_mac = None;
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
                session_mac.as_deref(),
                &session,
                link_status,
                &session_restore,
            )
            .await;
            if device.is_none() {
                *session_mac = None;
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
                session_mac.as_deref(),
                &session,
                link_status,
                &session_restore,
            )
            .await;
            if device.is_none() {
                *session_mac = None;
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
                session_mac.as_deref(),
                alarm_limits,
                monitor_polls,
                &session,
                link_status,
                &session_restore,
            )
            .await;
            if device.is_none() {
                *session_mac = None;
                *alarm_limits = None;
                *monitor_polls = 0;
                *session_restore = None;
            }
        }
        WorkerCommand::SetAlarmLimits(update) => {
            *device = handle_set_alarm_limits(
                events,
                device.take(),
                session_mac.as_deref(),
                update,
                alarm_limits,
                &session,
                link_status,
                &session_restore,
            )
            .await;
            if device.is_none() {
                *session_mac = None;
                *alarm_limits = None;
                *monitor_polls = 0;
                *link_status = DeviceStatus::default();
                *session_restore = None;
            }
        }
    }
    let _ = events.send(WorkerEvent::Busy(false));
}
