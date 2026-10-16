//! Odlaguna + Docker supervised run (M3 integration).
//!
//! `DOCKER_INTEGRATION=1 cargo test -p mimi-core --features docker-kill --test docker_skill_integration -- --ignored`

#![cfg(feature = "docker-kill")]

use std::time::Duration;

use mimi_core::{
    odlaguna::OdlagunaWatchdog, run_bollard_skill_supervised, OdlagunaDeadlineError,
};
use ryzu_runtime::{ResourceLimits, SkillRunRequest, SkillRunStatus};

fn enabled() -> bool {
    std::env::var("DOCKER_INTEGRATION")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[tokio::test]
#[ignore = "requires DOCKER_INTEGRATION=1"]
async fn odlaguna_deadline_kills_slow_skill() {
    if !enabled() {
        return;
    }

    let image = std::env::var("MIMI_WORKER_IMAGE").unwrap_or_else(|_| "mimi-worker:latest".into());
    let mut watchdog = OdlagunaWatchdog::new();
    let request = SkillRunRequest {
        image,
        command: vec!["sleep".into(), "60".into()],
        limits: ResourceLimits::skill_default(),
        network_isolated: true,
        timeout_ms: 3_000,
        heartbeat_interval_secs: None,
    };

    use mimi_core::InterventionContext;
    let err = run_bollard_skill_supervised(
        &mut watchdog,
        "odl-test",
        request,
        InterventionContext::default(),
    )
        .await
        .expect_err("should hit odlaguna deadline");

    assert!(matches!(err, OdlagunaDeadlineError::Timeout));
}

#[tokio::test]
#[ignore = "requires DOCKER_INTEGRATION=1"]
async fn supervised_echo_succeeds() {
    if !enabled() {
        return;
    }

    let image = std::env::var("MIMI_WORKER_IMAGE").unwrap_or_else(|_| "mimi-worker:latest".into());
    let mut watchdog = OdlagunaWatchdog::new();
    let request = SkillRunRequest {
        image,
        command: vec!["echo".into(), "supervised".into()],
        limits: ResourceLimits::skill_default(),
        network_isolated: true,
        timeout_ms: 30_000,
        heartbeat_interval_secs: Some(1),
    };

    let result = run_bollard_skill_supervised(
        &mut watchdog,
        "odl-ok",
        request,
        InterventionContext::default(),
    )
        .await
        .expect("run");

    assert_eq!(result.status, SkillRunStatus::Completed);
}
