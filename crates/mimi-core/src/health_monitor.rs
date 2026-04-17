//! Health Monitoring System
//!
//! Extends basic health checks with metric tracking, auto-publishing to Pandora,
//! and auto-escalation on threshold breaches.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

use crate::pandora_client::PandoraClient;
use crate::state_machine::{ComponentHealthCheck, MimiState};
use crate::zenoh_bus::ZenohBusAdapter;

const MAX_METRIC_HISTORY: usize = 1000;
const FAILURE_THRESHOLD: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetric {
    pub timestamp: DateTime<Utc>,
    pub component_name: String,
    pub metric_type: HealthMetricType,
    pub value: f64,
    pub threshold: f64,
    pub is_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthMetricType {
    CpuUsage,
    MemoryUsage,
    Latency,
    ErrorRate,
    HeartbeatMissed,
}

pub struct HealthMonitor {
    metrics: Arc<Mutex<VecDeque<HealthMetric>>>,
    pandora: Option<Arc<PandoraClient>>,
    zenoh: Option<Arc<ZenohBusAdapter>>,
    failure_counts: Arc<Mutex<std::collections::HashMap<String, usize>>>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_METRIC_HISTORY))),
            pandora: None,
            zenoh: None,
            failure_counts: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn with_pandora(mut self, pandora: Arc<PandoraClient>) -> Self {
        self.pandora = Some(pandora);
        self
    }

    pub async fn with_zenoh(mut self, zenoh: Arc<ZenohBusAdapter>) -> Self {
        self.zenoh = Some(zenoh);
        self
    }

    pub async fn record_metric(&self, metric: HealthMetric) -> Result<()> {
        {
            let mut metrics = self.metrics.lock().unwrap();
            if metrics.len() >= MAX_METRIC_HISTORY {
                metrics.pop_front();
            }
            metrics.push_back(metric.clone());
        }

        if !metric.is_healthy {
            self.track_failure(&metric.component_name).await?;
        } else {
            self.reset_failure_count(&metric.component_name).await;
        }

        if let Some(pandora) = &self.pandora {
            self.publish_to_pandora(pandora, &metric).await?;
        }

        if let Some(zenoh) = &self.zenoh {
            self.publish_to_zenoh(zenoh, &metric).await?;
        }

        Ok(())
    }

    async fn track_failure(&self, component: &str) -> Result<()> {
        let count = {
            let mut counts = self.failure_counts.lock().unwrap();
            let entry = counts.entry(component.to_string()).or_insert(0);
            *entry += 1;
            *entry
        };

        if count >= FAILURE_THRESHOLD {
            warn!(
                "Component {} exceeded failure threshold: {}/{}",
                component, count, FAILURE_THRESHOLD
            );
            self.escalate_failure(component).await?;
        }

        Ok(())
    }

    async fn reset_failure_count(&self, component: &str) {
        let mut counts = self.failure_counts.lock().unwrap();
        counts.remove(component);
    }

    async fn escalate_failure(&self, component: &str) -> Result<()> {
        info!("Escalating failure for component: {}", component);

        if let Some(pandora) = &self.pandora {
            let metadata = serde_json::json!({
                "component": component,
                "failure_count": FAILURE_THRESHOLD,
                "action": "escalated",
            });

            pandora
                .persist_critical_state(MimiState::FailedComponent, Utc::now(), metadata)
                .await?;
        }

        Ok(())
    }

    async fn publish_to_pandora(
        &self,
        pandora: &Arc<PandoraClient>,
        metric: &HealthMetric,
    ) -> Result<()> {
        if !metric.is_healthy {
            let metadata = serde_json::json!({
                "metric_type": format!("{:?}", metric.metric_type),
                "value": metric.value,
                "threshold": metric.threshold,
                "component": metric.component_name,
            });

            pandora
                .persist_critical_state(MimiState::Degraded, metric.timestamp, metadata)
                .await?;

            debug!(
                "Published unhealthy metric to Pandora: {:?}",
                metric.metric_type
            );
        }

        Ok(())
    }

    async fn publish_to_zenoh(
        &self,
        zenoh: &Arc<ZenohBusAdapter>,
        metric: &HealthMetric,
    ) -> Result<()> {
        if !metric.is_healthy {
            zenoh
                .publish_state_change(MimiState::Idle, MimiState::Degraded, metric.timestamp)
                .await?;

            debug!("Published health degradation to Zenoh");
        }

        Ok(())
    }

    pub fn get_recent_metrics(&self, count: usize) -> Vec<HealthMetric> {
        let metrics = self.metrics.lock().unwrap();
        metrics.iter().rev().take(count).cloned().collect()
    }

    pub fn get_metrics_in_window(&self, window_secs: i64) -> Vec<HealthMetric> {
        let metrics = self.metrics.lock().unwrap();
        let cutoff = Utc::now() - chrono::Duration::seconds(window_secs);

        metrics
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .cloned()
            .collect()
    }

    pub async fn check_component_health(&self, health_check: &ComponentHealthCheck) -> Result<()> {
        let is_healthy = health_check.is_healthy();

        let metric = HealthMetric {
            timestamp: Utc::now(),
            component_name: "system".to_string(),
            metric_type: HealthMetricType::ErrorRate,
            value: if is_healthy { 0.0 } else { 100.0 },
            threshold: 10.0,
            is_healthy,
        };

        self.record_metric(metric).await?;
        Ok(())
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_metric_tracking() {
        let monitor = HealthMonitor::new();

        let metric = HealthMetric {
            timestamp: Utc::now(),
            component_name: "test_component".to_string(),
            metric_type: HealthMetricType::CpuUsage,
            value: 45.0,
            threshold: 80.0,
            is_healthy: true,
        };

        let result = monitor.record_metric(metric).await;
        assert!(result.is_ok());

        let recent = monitor.get_recent_metrics(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].component_name, "test_component");
    }

    #[tokio::test]
    async fn test_auto_escalation() {
        let monitor = HealthMonitor::new();

        for _i in 0..FAILURE_THRESHOLD + 1 {
            let metric = HealthMetric {
                timestamp: Utc::now(),
                component_name: "failing_component".to_string(),
                metric_type: HealthMetricType::ErrorRate,
                value: 100.0,
                threshold: 10.0,
                is_healthy: false,
            };

            let result = monitor.record_metric(metric).await;
            assert!(result.is_ok());
        }

        let counts = monitor.failure_counts.lock().unwrap();
        assert!(counts.get("failing_component").unwrap_or(&0) >= &FAILURE_THRESHOLD);
    }

    #[tokio::test]
    async fn test_metrics_in_window() {
        let monitor = HealthMonitor::new();

        let old_metric = HealthMetric {
            timestamp: Utc::now() - chrono::Duration::hours(2),
            component_name: "test".to_string(),
            metric_type: HealthMetricType::Latency,
            value: 50.0,
            threshold: 100.0,
            is_healthy: true,
        };

        let new_metric = HealthMetric {
            timestamp: Utc::now(),
            component_name: "test".to_string(),
            metric_type: HealthMetricType::Latency,
            value: 60.0,
            threshold: 100.0,
            is_healthy: true,
        };

        monitor.record_metric(old_metric).await.unwrap();
        monitor.record_metric(new_metric).await.unwrap();

        let window_metrics = monitor.get_metrics_in_window(3600);
        assert_eq!(window_metrics.len(), 1);
    }

    #[tokio::test]
    async fn test_failure_count_reset() {
        let monitor = HealthMonitor::new();

        let bad_metric = HealthMetric {
            timestamp: Utc::now(),
            component_name: "test".to_string(),
            metric_type: HealthMetricType::ErrorRate,
            value: 100.0,
            threshold: 10.0,
            is_healthy: false,
        };

        monitor.record_metric(bad_metric).await.unwrap();

        {
            let counts = monitor.failure_counts.lock().unwrap();
            assert_eq!(counts.get("test"), Some(&1));
        }

        let good_metric = HealthMetric {
            timestamp: Utc::now(),
            component_name: "test".to_string(),
            metric_type: HealthMetricType::ErrorRate,
            value: 5.0,
            threshold: 10.0,
            is_healthy: true,
        };

        monitor.record_metric(good_metric).await.unwrap();

        {
            let counts = monitor.failure_counts.lock().unwrap();
            assert_eq!(counts.get("test"), None);
        }
    }
}
