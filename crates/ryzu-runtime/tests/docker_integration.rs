//! Docker integration tests (M3.1.4 / M3.2). Run with:
//! `DOCKER_INTEGRATION=1 cargo test -p ryzu-runtime --features docker --test docker_integration -- --ignored`

#[cfg(feature = "docker")]
mod docker_tests {
    use ryzu_runtime::{
        BollardSkillRuntime, ResourceLimits, SkillRunRequest, SkillRunStatus, SkillRuntime,
    };

    fn docker_integration_enabled() -> bool {
        std::env::var("DOCKER_INTEGRATION")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    fn worker_image() -> String {
        std::env::var("MIMI_WORKER_IMAGE").unwrap_or_else(|_| "mimi-worker:latest".into())
    }

    #[tokio::test]
    #[ignore = "requires DOCKER_INTEGRATION=1 and mimi-worker:latest"]
    async fn network_none_blocks_egress() {
        if !docker_integration_enabled() {
            return;
        }

        let rt = BollardSkillRuntime::connect().expect("docker");
        let result = rt
            .run_once(&SkillRunRequest {
                image: worker_image(),
                command: vec![
                    "sh".into(),
                    "-c".into(),
                    "curl -sf --max-time 2 https://1.1.1.1 >/dev/null 2>&1; echo CURL_FAILED=$?".into(),
                ],
                limits: ResourceLimits::skill_default(),
                network_isolated: true,
                timeout_ms: 15_000,
                heartbeat_interval_secs: None,
            })
            .await
            .expect("run");

        assert!(
            result.logs.contains("CURL_FAILED=0") == false,
            "expected curl to fail under network_mode=none, logs={}",
            result.logs
        );
    }

    #[tokio::test]
    #[ignore = "requires DOCKER_INTEGRATION=1 and mimi-worker:latest"]
    async fn skill_echo_completes() {
        if !docker_integration_enabled() {
            return;
        }

        let rt = BollardSkillRuntime::connect().expect("docker");
        let result = rt
            .run_once(&SkillRunRequest {
                image: worker_image(),
                command: vec!["echo".into(), "mimi-docker-ok".into()],
                limits: ResourceLimits::skill_default(),
                network_isolated: true,
                timeout_ms: 30_000,
                heartbeat_interval_secs: Some(1),
            })
            .await
            .expect("run");

        assert_eq!(result.status, SkillRunStatus::Completed);
        assert!(result.logs.contains("mimi-docker-ok"));
        assert!(result.logs.contains("mimi-heartbeat"));
    }

    #[tokio::test]
    #[ignore = "requires DOCKER_INTEGRATION=1 and mimi-worker:latest"]
    async fn short_timeout_kills_long_sleep() {
        if !docker_integration_enabled() {
            return;
        }

        let rt = BollardSkillRuntime::connect().expect("docker");
        let result = rt
            .run_once(&SkillRunRequest {
                image: worker_image(),
                command: vec!["sleep".into(), "120".into()],
                limits: ResourceLimits::skill_default(),
                network_isolated: true,
                timeout_ms: 2_000,
                heartbeat_interval_secs: None,
            })
            .await
            .expect("run");

        assert_eq!(result.status, SkillRunStatus::TimedOut);
    }
}
