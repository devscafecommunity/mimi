//! Concurrency limit for skill containers (M3.1.3 pool baseline).

use std::sync::Arc;

use tokio::sync::Semaphore;

pub struct SkillContainerPool {
    limit: Arc<Semaphore>,
}

impl SkillContainerPool {
    pub fn new(max_concurrent: u32) -> Self {
        Self {
            limit: Arc::new(Semaphore::new(max_concurrent.max(1) as usize)),
        }
    }

    pub async fn acquire(&self) -> tokio::sync::SemaphorePermit<'_> {
        self.limit
            .acquire()
            .await
            .expect("skill container pool semaphore closed")
    }
}

impl Default for SkillContainerPool {
    fn default() -> Self {
        Self::new(8)
    }
}
