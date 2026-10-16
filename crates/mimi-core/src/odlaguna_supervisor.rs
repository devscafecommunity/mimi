//! Global Odlaguna timer loop (100ms tick) for long-running / supervised workloads.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};

use crate::odlaguna::OdlagunaWatchdog;
use crate::odlaguna_intervention::InterventionContext;
use crate::pandora_client::PandoraClient;
use crate::skill_supervision::{intervene_on_expired, kill_containers_for_expired};
use crate::zenoh_bus::ZenohBusAdapter;

static GLOBAL_WATCHDOG: OnceLock<Arc<Mutex<OdlagunaWatchdog>>> = OnceLock::new();
static SUPERVISOR_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Shared watchdog polled by the background supervisor (daemon / scoped skill runs).
pub fn odlaguna_global() -> Arc<Mutex<OdlagunaWatchdog>> {
    GLOBAL_WATCHDOG
        .get_or_init(|| Arc::new(Mutex::new(OdlagunaWatchdog::new())))
        .clone()
}

pub fn odlaguna_supervisor_requested() -> bool {
    if std::env::var("MIMI_ODLAGUNA_SUPERVISOR")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return true;
    }
    std::env::var("MIMI_DAEMON")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn zenoh_requested() -> bool {
    std::env::var("MIMI_ZENOH")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn supervisor_tick_ms() -> u64 {
    std::env::var("ODLAGUNA_TICK_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
        .max(10)
}

/// Mirror lease to the global watchdog when the supervisor loop is active.
pub async fn mirror_register_if_active(
    task_id: &str,
    timeout: Duration,
    container_id: Option<String>,
) {
    if !SUPERVISOR_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let global = odlaguna_global();
    let mut wd = global.lock().await;
    wd.register_task_with_container(task_id, timeout, container_id);
}

pub async fn mirror_complete_if_active(task_id: &str) {
    if !SUPERVISOR_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let global = odlaguna_global();
    let mut wd = global.lock().await;
    wd.complete_task(task_id);
}

/// Background supervisor; stops on `stop()` / drop unless [`detach`](Self::detach) (daemon).
pub struct OdlagunaSupervisorLoop {
    handle: JoinHandle<()>,
    shutdown_tx: watch::Sender<bool>,
    keep_alive: bool,
}

impl OdlagunaSupervisorLoop {
    /// Do not stop the loop when this handle is dropped (use with `mimi daemon`).
    pub fn detach(mut self) -> Self {
        self.keep_alive = true;
        self
    }

    pub fn stop(mut self) {
        self.keep_alive = true;
        self.shutdown();
    }

    fn shutdown(&mut self) {
        SUPERVISOR_ACTIVE.store(false, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(true);
        self.handle.abort();
    }
}

impl Drop for OdlagunaSupervisorLoop {
    fn drop(&mut self) {
        if !self.keep_alive {
            self.shutdown();
        }
    }
}

/// Start the global timer wheel loop (call once per daemon, or scoped for isolated skills).
pub fn spawn_odlaguna_supervisor(
    pandora: Arc<PandoraClient>,
    zenoh: Option<Arc<ZenohBusAdapter>>,
) -> Result<OdlagunaSupervisorLoop> {
    if SUPERVISOR_ACTIVE.load(Ordering::SeqCst) {
        return Err(anyhow::anyhow!("Odlaguna supervisor already running"));
    }

    let watchdog = odlaguna_global();
    let tick = Duration::from_millis(supervisor_tick_ms());
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    SUPERVISOR_ACTIVE.store(true, Ordering::SeqCst);

    let handle = tokio::spawn(async move {
        let mut ticker = interval(tick);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        info!(
            target: "odlaguna.audit",
            tick_ms = tick.as_millis(),
            "Odlaguna supervisor loop started"
        );

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() && *shutdown_rx.borrow() {
                        info!(target: "odlaguna.audit", "Odlaguna supervisor loop stopped");
                        break;
                    }
                }
                _ = ticker.tick() => {
                    let expired = {
                        let mut wd = watchdog.lock().await;
                        wd.poll_expired()
                    };

                    if expired.is_empty() {
                        continue;
                    }

                    let ctx = InterventionContext {
                        pandora: Some(pandora.as_ref()),
                        zenoh: zenoh.as_deref(),
                    };
                    if let Err(e) = intervene_on_expired(&expired, ctx).await {
                        warn!("Odlaguna supervisor intervention failed: {}", e);
                    }
                    let _ = kill_containers_for_expired(&expired).await;
                }
            }
        }
    });

    Ok(OdlagunaSupervisorLoop {
        handle,
        shutdown_tx,
        keep_alive: false,
    })
}

/// Daemon entry: idempotent spawn (returns detached handle).
pub fn ensure_daemon_supervisor(
    pandora: Arc<PandoraClient>,
    zenoh: Option<Arc<ZenohBusAdapter>>,
) -> Result<OdlagunaSupervisorLoop> {
    static DAEMON_SUPERVISOR: OnceLock<()> = OnceLock::new();
    if DAEMON_SUPERVISOR.get().is_some() {
        return Err(anyhow::anyhow!("daemon supervisor already started"));
    }
    let loop_handle = spawn_odlaguna_supervisor(pandora, zenoh)?;
    let _ = DAEMON_SUPERVISOR.set(());
    Ok(loop_handle.detach())
}

pub fn supervisor_is_running() -> bool {
    SUPERVISOR_ACTIVE.load(Ordering::SeqCst)
}
