use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimi_core::serialization::{MessageSerializer, PROTOCOL_VERSION};

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_encode");

    for size in [64, 1024, 16 * 1024, 256 * 1024].iter() {
        let payload: Vec<u8> = vec![0xAB; *size];
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                MessageSerializer::encode_with_version(
                    PROTOCOL_VERSION,
                    "bench-msg",
                    1,
                    black_box(&payload),
                )
                .expect("encode")
            });
        });
    }
    group.finish();
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_decode");

    for size in [64, 1024, 16 * 1024, 256 * 1024].iter() {
        let payload: Vec<u8> = vec![0xAB; *size];
        let encoded = MessageSerializer::encode_with_version(
            PROTOCOL_VERSION,
            "bench-msg",
            1,
            &payload,
        )
        .expect("encode");

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                MessageSerializer::decode_with_version(black_box(&encoded)).expect("decode")
            });
        });
    }
    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    c.bench_function("serialization_roundtrip_1kb", |b| {
        let payload: Vec<u8> = vec![0xCD; 1024];
        b.iter(|| {
            let encoded = MessageSerializer::encode_with_version(
                PROTOCOL_VERSION,
                "roundtrip",
                2,
                black_box(&payload),
            )
            .expect("encode");
            let (_, id, ty, decoded) =
                MessageSerializer::decode_with_version(&encoded).expect("decode");
            black_box((id, ty, decoded));
        });
    });
}

criterion_group!(benches, bench_encode, bench_decode, bench_roundtrip);
criterion_main!(benches);
