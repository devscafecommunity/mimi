use ryzu_runtime::{RestartPolicy, ResourceLimits, SkillRunRequest, SkillRunResult, SkillRunStatus};

#[tokio::test]
async fn restart_policy_retries_failed_runs() {
    let policy = RestartPolicy {
        max_restarts: 2,
        base_backoff_ms: 1,
    };
    let request = SkillRunRequest {
        image: "img".into(),
        command: vec![],
        limits: ResourceLimits::skill_default(),
        network_isolated: true,
        timeout_ms: 1000,
        heartbeat_interval_secs: None,
    };

    let mut calls = 0u32;
    let result = ryzu_runtime::run_with_restart(&policy, &request, |_| {
        calls += 1;
        async move {
            Ok(SkillRunResult {
                container_id: "c".into(),
                status: if calls < 2 {
                    SkillRunStatus::Failed
                } else {
                    SkillRunStatus::Completed
                },
                exit_code: None,
                logs: String::new(),
            })
        }
    })
    .await
    .expect("ok");

    assert_eq!(result.status, SkillRunStatus::Completed);
    assert_eq!(calls, 2);
}
