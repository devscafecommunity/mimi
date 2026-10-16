//! Image pre-pull + concurrency cap (M3.1.3 warm pool baseline).

use std::collections::HashSet;
use std::sync::Mutex;

use anyhow::{Context, Result};
use tracing::info;

use crate::pool::SkillContainerPool;

#[cfg(feature = "docker")]
use bollard::image::CreateImageOptions;
#[cfg(feature = "docker")]
use bollard::Docker;
#[cfg(feature = "docker")]
use futures_util::StreamExt;

pub struct SkillWarmPool {
    pub concurrency: SkillContainerPool,
    warmed: Mutex<HashSet<String>>,
}

impl SkillWarmPool {
    pub fn new(max_concurrent: u32) -> Self {
        Self {
            concurrency: SkillContainerPool::new(max_concurrent),
            warmed: Mutex::new(HashSet::new()),
        }
    }

    pub async fn acquire(&self) -> tokio::sync::SemaphorePermit<'_> {
        self.concurrency.acquire().await
    }

    #[cfg(feature = "docker")]
    pub async fn prewarm_image(&self, docker: &Docker, image: &str) -> Result<()> {
        {
            let warmed = self.warmed.lock().unwrap();
            if warmed.contains(image) {
                return Ok(());
            }
        }

        info!(target: "ryzu.audit", image = %image, "prewarming worker image");
        let (from_image, tag) = if let Some((name, tag)) = image.rsplit_once(':') {
            (name, tag)
        } else {
            (image, "latest")
        };
        let mut stream = docker.create_image(
            Some(CreateImageOptions {
                from_image,
                from_src: "",
                repo: "",
                tag,
                platform: "",
                changes: Vec::new(),
            }),
            None,
            None,
        );

        while let Some(update) = stream.next().await {
            update.context("pull image")?;
        }

        self.warmed.lock().unwrap().insert(image.to_string());
        Ok(())
    }

    #[cfg(not(feature = "docker"))]
    pub async fn prewarm_image(&self, _docker: &(), _image: &str) -> Result<()> {
        Ok(())
    }
}

impl Default for SkillWarmPool {
    fn default() -> Self {
        Self::new(8)
    }
}
