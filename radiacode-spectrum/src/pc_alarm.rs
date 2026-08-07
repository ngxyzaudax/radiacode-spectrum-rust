use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tracing::debug;

static LAST_BEEP: Mutex<Option<Instant>> = Mutex::new(None);

pub fn maybe_beep_alarm(enabled: bool) {
    if !enabled {
        return;
    }
    let Ok(mut last) = LAST_BEEP.lock() else {
        return;
    };
    if last.is_some_and(|instant| instant.elapsed() < Duration::from_secs(2)) {
        return;
    }
    *last = Some(Instant::now());
    if Command::new("canberra-gtk-play")
        .args(["-i", "message"])
        .spawn()
        .is_ok()
    {
        debug!("pc alarm beep via canberra");
        return;
    }
    let _ = Command::new("paplay")
        .arg("/usr/share/sounds/freedesktop/stereo/message.oga")
        .spawn();
    debug!("pc alarm beep attempted");
}
