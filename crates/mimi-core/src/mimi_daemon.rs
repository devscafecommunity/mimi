//! Long-running MiMi daemon: Pandora flush + Odlaguna supervisor (M2.6 / M3).

use std::sync::Arc;

use anyhow::Result;
use tokio::signal;
use tracing::info;

use crate::intervention_setup::{connect_zenoh_if_requested, start_daemon_odlaguna_supervisor};
use crate::odlaguna_supervisor::odlaguna_supervisor_requested;
use crate::pandora_client::PandoraClient;
use crate::pandora_integration::{checkpoint_interval_from_env, flush_interval_from_env, PandoraFlushLoop};
use crate::state_machine::StateManager;

/// Run maintenance loops until SIGINT / SIGTERM.
pub async fn run_daemon() -> Result<()> {
    std::env::set_var("MIMI_DAEMON", "1");

    let pandora = Arc::new(
        PandoraClient::new()
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?,
    );
    let state = Arc::new(StateManager::new());

    let zenoh = connect_zenoh_if_requested().await?;

    let odlaguna_loop = start_daemon_odlaguna_supervisor(pandora.clone(), zenoh.clone())?;

    let flush_period = flush_interval_from_env();
    let _checkpoint_period = checkpoint_interval_from_env();
    let (flush_loop, shutdown) =
        PandoraFlushLoop::spawn_with_checkpoint(pandora.clone(), Some(state.clone()), flush_period);

    info!(
        "MiMi daemon running (flush={}s, odlaguna_supervisor={}, zenoh={})",
        flush_period.as_secs(),
        odlaguna_supervisor_requested() || crate::odlaguna_supervisor::supervisor_is_running(),
        zenoh.is_some()
    );

    signal::ctrl_c().await?;
    info!("MiMi daemon shutdown requested");
    let _ = shutdown.send(true);
    flush_loop.stop().await;
    odlaguna_loop.stop();

    Ok(())
}
