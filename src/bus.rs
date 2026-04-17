use crate::protocol::{Message, MessageBodyType};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use zenoh::config::Config;
use zenoh::prelude::*;

pub mod config {
    use super::*;

    #[derive(Clone, Debug)]
    pub struct ConnectionConfig {
        pub broker_endpoint: String,
        pub timeout_ms: u64,
        pub max_message_size: usize,
        pub connection_pool_size: usize,
    }

    impl Default for ConnectionConfig {
        fn default() -> Self {
            Self {
                broker_endpoint: "tcp/127.0.0.1:7447".to_string(),
                timeout_ms: 5000,
                max_message_size: 4 * 1024 * 1024,
                connection_pool_size: 4,
            }
        }
    }
}

pub mod pool {
    use super::*;
    use zenoh::Session;

    pub struct ConnectionPool {
        sessions: Vec<Arc<Session>>,
        current_index: Arc<RwLock<usize>>,
    }

    impl ConnectionPool {
        pub async fn new(cfg: &config::ConnectionConfig) -> Result<Self> {
            let mut sessions = Vec::with_capacity(cfg.connection_pool_size);

            for _ in 0..cfg.connection_pool_size {
                let zenoh_config = Self::build_zenoh_config(&cfg.broker_endpoint)?;
                let session = zenoh::open(zenoh_config)
                    .await
                    .map_err(|e| anyhow!("Failed to open Zenoh session: {}", e))?;

                sessions.push(Arc::new(session));
            }

            Ok(Self {
                sessions,
                current_index: Arc::new(RwLock::new(0)),
            })
        }

        pub async fn get_session(&self) -> Arc<Session> {
            let mut idx = self.current_index.write().await;
            let session = self.sessions[*idx].clone();
            *idx = (*idx + 1) % self.sessions.len();
            session
        }

        fn build_zenoh_config(endpoint: &str) -> Result<Config> {
            let mut config = Config::default();
            config
                .insert_json5("connect/endpoints", &format!(r#"["{}"]"#, endpoint))
                .map_err(|e| anyhow!("Failed to configure Zenoh: {}", e))?;

            Ok(config)
        }

        pub async fn close(&self) -> Result<()> {
            for session in &self.sessions {
                session
                    .close()
                    .await
                    .map_err(|e| anyhow!("Failed to close session: {}", e))?;
            }
            Ok(())
        }
    }
}

pub mod publisher {
    use super::*;
    use zenoh::prelude::*;

    pub struct Publisher {
        pool: Arc<pool::ConnectionPool>,
        topic: String,
        reliable: bool,
    }

    impl Publisher {
        pub fn new(pool: Arc<pool::ConnectionPool>, topic: String, reliable: bool) -> Self {
            Self {
                pool,
                topic,
                reliable,
            }
        }

        pub async fn publish(&self, data: &[u8]) -> Result<()> {
            let session = self.pool.get_session().await;

            let reliability = if self.reliable {
                Reliability::Reliable
            } else {
                Reliability::BestEffort
            };

            session
                .put(&self.topic, data)
                .reliability(reliability)
                .await
                .map_err(|e| anyhow!("Publish failed: {}", e))?;

            Ok(())
        }

        pub async fn publish_with_congestion_control(
            &self,
            data: &[u8],
            congestion_control: CongestionControl,
        ) -> Result<()> {
            let session = self.pool.get_session().await;

            let reliability = if self.reliable {
                Reliability::Reliable
            } else {
                Reliability::BestEffort
            };

            session
                .put(&self.topic, data)
                .reliability(reliability)
                .congestion_control(congestion_control)
                .await
                .map_err(|e| anyhow!("Publish with CC failed: {}", e))?;

            Ok(())
        }
    }
}

pub mod subscriber {
    use super::*;
    use tokio::sync::mpsc;
    use zenoh::prelude::*;

    pub struct Subscriber {
        _task: tokio::task::JoinHandle<()>,
        rx: mpsc::Receiver<Vec<u8>>,
    }

    impl Subscriber {
        pub async fn new(
            pool: Arc<pool::ConnectionPool>,
            topic: String,
            buffer_size: usize,
        ) -> Result<Self> {
            let session = pool.get_session().await;

            let (tx, rx) = mpsc::channel(buffer_size);

            let sub = session
                .declare_subscriber(&topic)
                .await
                .map_err(|e| anyhow!("Failed to declare subscriber: {}", e))?;

            let task = tokio::spawn(async move {
                while let Ok(sample) = sub.recv_async().await {
                    let data = sample.payload().to_bytes().to_vec();
                    let _ = tx.send(data).await;
                }
            });

            Ok(Self { _task: task, rx })
        }

        pub async fn recv(&mut self) -> Option<Vec<u8>> {
            self.rx.recv().await
        }

        pub async fn recv_timeout(&mut self, timeout: Duration) -> Option<Vec<u8>> {
            tokio::time::timeout(timeout, self.rx.recv())
                .await
                .ok()
                .flatten()
        }
    }
}

pub mod request_reply {
    use super::*;
    use zenoh::prelude::*;

    pub struct Replier {
        pool: Arc<pool::ConnectionPool>,
        topic: String,
        handler: Arc<dyn Fn(Vec<u8>) -> anyhow::Result<Vec<u8>> + Send + Sync>,
    }

    impl Replier {
        pub fn new<F>(pool: Arc<pool::ConnectionPool>, topic: String, handler: F) -> Self
        where
            F: Fn(Vec<u8>) -> anyhow::Result<Vec<u8>> + Send + Sync + 'static,
        {
            Self {
                pool,
                topic,
                handler: Arc::new(handler),
            }
        }

        pub async fn start(&self) -> Result<()> {
            let session = self.pool.get_session().await;
            let handler = self.handler.clone();

            let _queryable = session
                .declare_queryable(&self.topic)
                .await
                .map_err(|e| anyhow!("Failed to declare queryable: {}", e))?;

            let _task = tokio::spawn(async move {
                let mut queryable = session.declare_queryable(&self.topic).await.ok()?;

                while let Ok(query) = queryable.recv_async().await {
                    let request = query.payload().to_bytes().to_vec();
                    match handler(request) {
                        Ok(response) => {
                            let _ = query.reply(Value::from(response)).await;
                        },
                        Err(_e) => {
                            let _ = query.reply(Value::from("error")).await;
                        },
                    }
                }

                Some(())
            });

            Ok(())
        }
    }

    pub struct Requester {
        pool: Arc<pool::ConnectionPool>,
        topic: String,
        timeout: Duration,
    }

    impl Requester {
        pub fn new(pool: Arc<pool::ConnectionPool>, topic: String, timeout: Duration) -> Self {
            Self {
                pool,
                topic,
                timeout,
            }
        }

        pub async fn request(&self, data: &[u8]) -> Result<Vec<u8>> {
            let session = self.pool.get_session().await;

            let selector = Selector::from(&self.topic);

            let responses = session
                .get(selector)
                .with(Value::from(data.to_vec()))
                .await
                .map_err(|e| anyhow!("Request failed: {}", e))?;

            tokio::time::timeout(self.timeout, async {
                while let Ok(response) = responses.recv_async().await {
                    return Ok::<_, anyhow::Error>(response.sample.payload().to_bytes().to_vec());
                }
                Err(anyhow!("No response received"))
            })
            .await
            .map_err(|_| anyhow!("Request timeout"))?
        }
    }
}

pub mod client {
    use super::*;

    pub struct ZenohClient {
        pool: Arc<pool::ConnectionPool>,
        config: config::ConnectionConfig,
    }

    impl ZenohClient {
        pub async fn new(cfg: config::ConnectionConfig) -> Result<Self> {
            let pool = Arc::new(pool::ConnectionPool::new(&cfg).await?);
            Ok(Self { pool, config: cfg })
        }

        pub fn publisher(&self, topic: String, reliable: bool) -> publisher::Publisher {
            publisher::Publisher::new(self.pool.clone(), topic, reliable)
        }

        pub async fn subscriber(
            &self,
            topic: String,
            buffer_size: usize,
        ) -> Result<subscriber::Subscriber> {
            subscriber::Subscriber::new(self.pool.clone(), topic, buffer_size).await
        }

        pub fn replier<F>(&self, topic: String, handler: F) -> request_reply::Replier
        where
            F: Fn(Vec<u8>) -> anyhow::Result<Vec<u8>> + Send + Sync + 'static,
        {
            request_reply::Replier::new(self.pool.clone(), topic, handler)
        }

        pub fn requester(&self, topic: String, timeout: Duration) -> request_reply::Requester {
            request_reply::Requester::new(self.pool.clone(), topic, timeout)
        }

        pub async fn close(&self) -> Result<()> {
            self.pool.close().await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_connection_pool_creation() {
        let cfg = config::ConnectionConfig::default();
        let result = pool::ConnectionPool::new(&cfg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn test_publisher_creation() {
        let cfg = config::ConnectionConfig::default();
        let pool = Arc::new(pool::ConnectionPool::new(&cfg).await.unwrap());
        let pub_topic = "test/topic".to_string();

        let publisher = publisher::Publisher::new(pool, pub_topic, true);
        let result = publisher.publish(b"test data").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_defaults() {
        let cfg = config::ConnectionConfig::default();
        assert_eq!(cfg.broker_endpoint, "tcp/127.0.0.1:7447");
        assert_eq!(cfg.timeout_ms, 5000);
        assert_eq!(cfg.connection_pool_size, 4);
    }
}
