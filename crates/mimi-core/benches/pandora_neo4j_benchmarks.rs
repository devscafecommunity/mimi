//! Neo4j Pandora latency benchmarks (M2.7.3 / acceptance).
//!
//! Run with live Neo4j:
//!   MIMI_BENCH_NEO4J=1 cargo bench -p mimi-core --bench pandora_neo4j_benchmarks

use std::time::Instant;

use criterion::{criterion_group, criterion_main, Criterion};
use mimi_core::{Neo4jConfig, PandoraClient};

async fn bench_client() -> Option<PandoraClient> {
    if std::env::var("MIMI_BENCH_NEO4J").is_err() {
        return None;
    }
    let client = PandoraClient::with_config(Neo4jConfig::from_env())
        .await
        .ok()?;
    if !client.is_neo4j_connected() {
        return None;
    }
    Some(client)
}

fn bench_touch_context(c: &mut Criterion) {
    let mut group = c.benchmark_group("pandora_neo4j_touch");
    group.sample_size(20);

    group.bench_function("touch_context", |b| {
        b.iter_custom(|iters| {
            let rt = tokio::runtime::Runtime::new().expect("runtime");
            rt.block_on(async {
                let Some(client) = bench_client().await else {
                    return std::time::Duration::from_secs(0);
                };
                let start = Instant::now();
                for i in 0..iters {
                    let id = format!("ctx:bench-{}", i);
                    client
                        .touch_context(&id, Some("bench"))
                        .await
                        .expect("touch");
                }
                start.elapsed()
            })
        });
    });

    group.finish();
}

criterion_group!(benches, bench_touch_context);
criterion_main!(benches);
