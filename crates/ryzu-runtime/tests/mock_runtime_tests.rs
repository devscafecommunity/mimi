use ryzu_runtime::{
    MockSkillRuntime, ResourceLimits, SkillRunRequest, SkillRunStatus, SkillRuntime,
};

#[tokio::test]
async fn mock_skill_run_completes() {
    let rt = MockSkillRuntime::new();
    let result = rt
        .run(SkillRunRequest {
            image: "mimi-worker:latest".into(),
            command: vec!["echo".into(), "ok".into()],
            limits: ResourceLimits::skill_default(),
            network_isolated: true,
            timeout_ms: 5_000,
            heartbeat_interval_secs: None,
        })
        .await
        .expect("run");

    assert_eq!(result.status, SkillRunStatus::Completed);
    assert_eq!(result.exit_code, Some(0));
}

#[test]
fn resource_limits_validation() {
    let bad = ResourceLimits {
        memory_mb: 0,
        cpu_percent: 50,
    };
    assert!(bad.validate().is_err());
}
