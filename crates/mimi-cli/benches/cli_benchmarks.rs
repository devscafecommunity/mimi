use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimi_cli::auth::{AuthManager, Identity, Role};
use std::collections::HashSet;

/// Benchmark token generation performance
fn bench_token_generation(c: &mut Criterion) {
    c.bench_function("token_generation_single", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);

            let mut roles = HashSet::new();
            roles.insert(Role::User);

            let identity = Identity {
                user_id: black_box("user-1".to_string()),
                username: black_box("alice".to_string()),
                roles,
            };

            let _ = auth_manager.generate_token(&identity);
        });
    });

    let mut group = c.benchmark_group("token_generation_batch");
    group.sample_size(10);

    for batch_size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let auth_manager = AuthManager::new("secret".to_string(), 3600);
                    let mut roles = HashSet::new();
                    roles.insert(Role::User);

                    for i in 0..batch_size {
                        let identity = Identity {
                            user_id: format!("user-{}", i),
                            username: format!("user{}", i),
                            roles: roles.clone(),
                        };
                        let _ = auth_manager.generate_token(black_box(&identity));
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark token validation performance
fn bench_token_validation(c: &mut Criterion) {
    c.bench_function("token_validation_single", |b| {
        b.iter_batched(
            || {
                let auth_manager = AuthManager::new("secret".to_string(), 3600);
                let mut roles = HashSet::new();
                roles.insert(Role::User);

                let identity = Identity {
                    user_id: "user-1".to_string(),
                    username: "alice".to_string(),
                    roles,
                };

                let token = auth_manager.generate_token(&identity).unwrap();
                (auth_manager, token)
            },
            |(auth_manager, token)| {
                let _ = auth_manager.validate_token(&token);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let mut group = c.benchmark_group("token_validation_batch");
    group.sample_size(10);

    for batch_size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let auth_manager = AuthManager::new("secret".to_string(), 3600);
                        let mut roles = HashSet::new();
                        roles.insert(Role::User);

                        let mut tokens = Vec::new();
                        for i in 0..batch_size {
                            let identity = Identity {
                                user_id: format!("user-{}", i),
                                username: format!("user{}", i),
                                roles: roles.clone(),
                            };
                            let token = auth_manager.generate_token(&identity).unwrap();
                            tokens.push(token);
                        }
                        (auth_manager, tokens)
                    },
                    |(auth_manager, tokens)| {
                        for token in &tokens {
                            let _ = auth_manager.validate_token(black_box(token));
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmark permission checking performance
fn bench_permission_checks(c: &mut Criterion) {
    c.bench_function("permission_check_single", |b| {
        b.iter_batched(
            || {
                let auth_manager = AuthManager::new("secret".to_string(), 3600);
                auth_manager.register_default_policies();

                let mut roles = HashSet::new();
                roles.insert(Role::User);

                let identity = Identity {
                    user_id: "user-1".to_string(),
                    username: "alice".to_string(),
                    roles,
                };

                (auth_manager, identity)
            },
            |(auth_manager, identity)| {
                let _ = auth_manager.check_permission(&identity, "query", "read");
            },
            criterion::BatchSize::SmallInput,
        );
    });

    let mut group = c.benchmark_group("permission_check_batch");
    group.sample_size(10);

    for batch_size in [1000, 5000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter_batched(
                    || {
                        let auth_manager = AuthManager::new("secret".to_string(), 3600);
                        auth_manager.register_default_policies();

                        let mut roles = HashSet::new();
                        roles.insert(Role::User);

                        let identity = Identity {
                            user_id: "user-1".to_string(),
                            username: "alice".to_string(),
                            roles,
                        };

                        (auth_manager, identity)
                    },
                    |(auth_manager, identity)| {
                        for i in 0..batch_size {
                            let resource = if i % 2 == 0 { "query" } else { "execute" };
                            let _ = auth_manager.check_permission(
                                black_box(&identity),
                                black_box(resource),
                                black_box("read"),
                            );
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmark AuthManager creation and initialization
fn bench_auth_manager_creation(c: &mut Criterion) {
    c.bench_function("auth_manager_creation", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);
            auth_manager.register_default_policies();
            black_box(auth_manager);
        });
    });

    c.bench_function("auth_manager_creation_with_custom_ttl", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("custom-secret".to_string(), 7200);
            auth_manager.register_default_policies();
            black_box(auth_manager);
        });
    });
}

criterion_group!(
    benches,
    bench_token_generation,
    bench_token_validation,
    bench_permission_checks,
    bench_auth_manager_creation
);

criterion_main!(benches);
