use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mimi_core::state_machine::MimiState;
use mimi_core::zenoh_bus::ZenohBusAdapter;
use tokio::runtime::Runtime;

fn bench_zenoh_adapter(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");

    c.bench_function("zenoh_adapter_create", |b| {
        b.iter(|| {
            rt.block_on(async {
                let adapter = ZenohBusAdapter::new().await.expect("adapter");
                black_box(adapter);
            });
        });
    });

    c.bench_function("zenoh_publish_state_change", |b| {
        b.iter(|| {
            rt.block_on(async {
                let adapter = ZenohBusAdapter::new().await.expect("adapter");
                adapter
                    .publish_state_change(
                        MimiState::Idle,
                        MimiState::Listening,
                        Utc::now(),
                    )
                    .await
                    .expect("publish");
            });
        });
    });

    c.bench_function("zenoh_subscribe_tasks", |b| {
        b.iter(|| {
            rt.block_on(async {
                let adapter = ZenohBusAdapter::new().await.expect("adapter");
                let rx = adapter.subscribe_tasks().await.expect("subscribe");
                black_box(rx);
            });
        });
    });
}

criterion_group!(benches, bench_zenoh_adapter);
criterion_main!(benches);
