//! Wire Pandora + Zenoh + Odlaguna supervisor for CLI/daemon paths.

use std::sync::Arc;

use anyhow::Result;

use crate::odlaguna_intervention::InterventionContext;
use crate::odlaguna_supervisor::{
    odlaguna_supervisor_requested, spawn_odlaguna_supervisor, zenoh_requested,
    OdlagunaSupervisorLoop,
};
use crate::pandora_client::PandoraClient;
use crate::zenoh_bus::ZenohBusAdapter;

/// Optional Zenoh adapter when `MIMI_ZENOH=1`.
pub async fn connect_zenoh_if_requested() -> Result<Option<Arc<ZenohBusAdapter>>> {
    if !zenoh_requested() {
        return Ok(None);
    }
    let adapter = ZenohBusAdapter::from_env().await?;
    Ok(Some(Arc::new(adapter)))
}

/// Scoped supervisor for isolated Docker skills (stops when the returned handle drops).
pub fn start_scoped_odlaguna_supervisor(
    pandora: Arc<PandoraClient>,
    zenoh: Option<Arc<ZenohBusAdapter>>,
) -> Result<OdlagunaSupervisorLoop> {
    spawn_odlaguna_supervisor(pandora, zenoh)
}

/// Daemon-only supervisor (detached; stop explicitly on shutdown).
pub fn start_daemon_odlaguna_supervisor(
    pandora: Arc<PandoraClient>,
    zenoh: Option<Arc<ZenohBusAdapter>>,
) -> Result<OdlagunaSupervisorLoop> {
    use crate::odlaguna_supervisor::ensure_daemon_supervisor;
    ensure_daemon_supervisor(pandora, zenoh)
}

/// Env-gated scoped supervisor (`MIMI_ODLAGUNA_SUPERVISOR=1`).
pub fn start_odlaguna_supervisor_if_requested(
    pandora: Arc<PandoraClient>,
    zenoh: Option<Arc<ZenohBusAdapter>>,
) -> Result<Option<OdlagunaSupervisorLoop>> {
    if !odlaguna_supervisor_requested() {
        return Ok(None);
    }
    spawn_odlaguna_supervisor(pandora, zenoh).map(Some)
}

pub fn intervention_context<'a>(
    pandora: &'a PandoraClient,
    zenoh: Option<&'a ZenohBusAdapter>,
) -> InterventionContext<'a> {
    InterventionContext {
        pandora: Some(pandora),
        zenoh,
    }
}
