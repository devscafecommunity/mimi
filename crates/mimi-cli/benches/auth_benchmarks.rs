use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimi_cli::auth::{AuthManager, Identity, Role};
use std::collections::HashSet;

/// Benchmark identity creation and token lifecycle
fn bench_identity_to_token_lifecycle(c: &mut Criterion) {
    c.bench_function("identity_to_token_lifecycle", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);

            let mut roles = HashSet::new();
            roles.insert(Role::Admin);

            let identity = Identity {
                user_id: black_box("admin-user".to_string()),
                username: black_box("alice".to_string()),
                roles,
            };

            let token = auth_manager.generate_token(&identity).unwrap();
            let validated_identity = auth_manager.validate_token(&token).unwrap();

            black_box(validated_identity);
        });
    });
}

/// Benchmark role hierarchy lookup performance
fn bench_role_hierarchy_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("role_hierarchy");

    for role in [Role::Guest, Role::User, Role::Admin].iter() {
        let role_clone = role.clone();
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", role)),
            &role_clone,
            |b, role| {
                b.iter(|| {
                    let mut roles = HashSet::new();
                    roles.insert(role.clone());

                    let identity = Identity {
                        user_id: "user-1".to_string(),
                        username: "test".to_string(),
                        roles,
                    };

                    black_box(identity);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark identity lookup with multiple roles
fn bench_multi_role_identity(c: &mut Criterion) {
    c.bench_function("multi_role_identity_lookup", |b| {
        b.iter(|| {
            let mut roles = HashSet::new();
            roles.insert(Role::Admin);
            roles.insert(Role::User);

            let identity = Identity {
                user_id: black_box("power-user".to_string()),
                username: black_box("alice".to_string()),
                roles,
            };

            black_box(identity);
        });
    });
}

/// Benchmark identity registration and retrieval
fn bench_identity_registry(c: &mut Criterion) {
    c.bench_function("identity_registration_single", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);

            let mut roles = HashSet::new();
            roles.insert(Role::User);

            let identity = Identity {
                user_id: black_box("user-1".to_string()),
                username: black_box("alice".to_string()),
                roles,
            };

            auth_manager.register_identity(identity.clone());
            let _ = auth_manager.get_identity(&identity.user_id);

            black_box(auth_manager);
        });
    });

    let mut group = c.benchmark_group("identity_registry_batch");
    group.sample_size(10);

    for identity_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(identity_count),
            identity_count,
            |b, &identity_count| {
                b.iter(|| {
                    let auth_manager = AuthManager::new("secret".to_string(), 3600);

                    for i in 0..identity_count {
                        let mut roles = HashSet::new();
                        roles.insert(Role::User);

                        let identity = Identity {
                            user_id: format!("user-{}", i),
                            username: format!("user{}", i),
                            roles,
                        };

                        auth_manager.register_identity(identity);
                    }

                    black_box(auth_manager);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark permission check with various policy sizes
fn bench_permission_check_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("permission_check_scaling");
    group.sample_size(10);

    for policy_count in [5, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(policy_count),
            policy_count,
            |b, &policy_count| {
                b.iter_batched(
                    || {
                        let auth_manager = AuthManager::new("secret".to_string(), 3600);
                        auth_manager.register_default_policies();

                        let mut roles = HashSet::new();
                        roles.insert(Role::User);

                        let identity = Identity {
                            user_id: "user-1".to_string(),
                            username: "test".to_string(),
                            roles,
                        };

                        (auth_manager, identity)
                    },
                    |(auth_manager, identity)| {
                        for i in 0..policy_count {
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

/// Benchmark token generation with different role combinations
fn bench_token_generation_roles(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_generation_roles");

    group.bench_function("token_gen_single_role", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);
            let mut roles = HashSet::new();
            roles.insert(Role::User);

            let identity = Identity {
                user_id: "user-1".to_string(),
                username: "alice".to_string(),
                roles,
            };

            let _ = auth_manager.generate_token(black_box(&identity));
        });
    });

    group.bench_function("token_gen_multi_role", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);
            let mut roles = HashSet::new();
            roles.insert(Role::Admin);
            roles.insert(Role::User);

            let identity = Identity {
                user_id: "power-user".to_string(),
                username: "alice".to_string(),
                roles,
            };

            let _ = auth_manager.generate_token(black_box(&identity));
        });
    });

    group.finish();
}

/// Benchmark token validation with various token states
fn bench_token_validation_variants(c: &mut Criterion) {
    c.bench_function("token_validation_valid", |b| {
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
                let _ = auth_manager.validate_token(black_box(&token));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark concurrent auth operations simulation
fn bench_concurrent_auth_operations(c: &mut Criterion) {
    c.bench_function("sequential_token_gen_and_validate", |b| {
        b.iter(|| {
            let auth_manager = AuthManager::new("secret".to_string(), 3600);
            let mut roles = HashSet::new();
            roles.insert(Role::User);

            for i in 0..10 {
                let identity = Identity {
                    user_id: format!("user-{}", i),
                    username: format!("user{}", i),
                    roles: roles.clone(),
                };

                let token = auth_manager.generate_token(&identity).unwrap();
                let _ = auth_manager.validate_token(black_box(&token));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_identity_to_token_lifecycle,
    bench_role_hierarchy_lookup,
    bench_multi_role_identity,
    bench_identity_registry,
    bench_permission_check_scaling,
    bench_token_generation_roles,
    bench_token_validation_variants,
    bench_concurrent_auth_operations
);

criterion_main!(benches);
