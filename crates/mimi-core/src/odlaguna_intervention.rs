//! Odlaguna post-intervention hooks: Neo4j audit + Zenoh (M3.2.6 / bus).

use anyhow::Result;

use crate::odlaguna::ExpiredTask;
use crate::pandora_client::PandoraClient;
use crate::zenoh_bus::ZenohBusAdapter;

#[derive(Default)]
pub struct InterventionContext<'a> {
    pub pandora: Option<&'a PandoraClient>,
    pub zenoh: Option<&'a ZenohBusAdapter>,
}

/// Persist audit + bus notifications for expired tasks (after `poll_expired`).
pub async fn emit_intervention_side_effects(
    expired: &[ExpiredTask],
    ctx: InterventionContext<'_>,
) -> Result<()> {
    for task in expired {
        if let Some(z) = ctx.zenoh {
            z.publish_task_timeout(&task.task_id, task.container_id.as_deref())
                .await?;
        }
        if let Some(p) = ctx.pandora {
            p.record_odlaguna_intervention(
                &task.task_id,
                "timeout",
                "deadline exceeded — intervention",
                task.container_id.as_deref(),
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::odlaguna::ExpiredTask;

    #[tokio::test]
    async fn audit_and_zenoh_hooks_run() {
        let pandora = PandoraClient::new().await.expect("pandora");
        let zenoh = ZenohBusAdapter::new().await.expect("zenoh");
        let expired = vec![ExpiredTask {
            task_id: "t-audit".into(),
            container_id: Some("cid-1".into()),
        }];
        emit_intervention_side_effects(
            &expired,
            InterventionContext {
                pandora: Some(&pandora),
                zenoh: Some(&zenoh),
            },
        )
        .await
        .expect("side effects");

        let log = pandora.odlaguna_audit_log().await;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].task_id, "t-audit");
        assert_eq!(log[0].action, "timeout");
    }
}
