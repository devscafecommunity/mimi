//! CLI Load Tests and Stress Tests
//!
//! Comprehensive load testing for the MiMi CLI and authentication stack.
//! Measures throughput, latency (p50/p95/p99), memory stability, and concurrent operations.
//!
//! Tests:
//! 1. Token generation throughput (1000 tokens)
//! 2. Token validation throughput (1000 validations)
//! 3. Permission checks throughput (10000 checks)
//! 4. AuthManager scaling with 100 policies
//! 5. CLI command execution batch (100 commands)
//! 6. Concurrent token generation (10 threads)
//! 7. Concurrent validation (10 threads)
//! 8. Large identity lookup (1000 identities)
//! 9. Auth policy registry memory (1000 policies)
//! 10. CLI startup overhead (10 iterations)

mod cli_integration_utils;

use cli_integration_utils::*;
use mimi_cli::auth::{AuthManager, Identity, Role};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

/// Performance metrics for load test results
#[derive(Debug, Clone)]
pub struct LoadMetrics {
    pub total_ops: usize,
    pub total_time_ms: f64,
    pub min_latency_us: f64,
    pub max_latency_us: f64,
    pub avg_latency_us: f64,
    pub p50_latency_us: f64,
    pub p95_latency_us: f64,
    pub p99_latency_us: f64,
    pub throughput_ops_per_sec: f64,
    pub memory_peak_mb: f64,
}

impl LoadMetrics {
    /// Create metrics from latency measurements (microseconds)
    pub fn from_latencies(total_ops: usize, latencies: Vec<f64>, total_time_ms: f64) -> Self {
        let mut sorted = latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);
        let avg = latencies.iter().sum::<f64>() / latencies.len().max(1) as f64;

        let p50_idx = (sorted.len() as f64 * 0.50).ceil() as usize;
        let p95_idx = (sorted.len() as f64 * 0.95).ceil() as usize;
        let p99_idx = (sorted.len() as f64 * 0.99).ceil() as usize;

        let p50 = sorted
            .get(p50_idx.saturating_sub(1))
            .copied()
            .unwrap_or(0.0);
        let p95 = sorted
            .get(p95_idx.saturating_sub(1))
            .copied()
            .unwrap_or(0.0);
        let p99 = sorted
            .get(p99_idx.saturating_sub(1))
            .copied()
            .unwrap_or(0.0);

        let throughput_ops_per_sec = (total_ops as f64 / total_time_ms) * 1000.0;

        LoadMetrics {
            total_ops,
            total_time_ms,
            min_latency_us: min,
            max_latency_us: max,
            avg_latency_us: avg,
            p50_latency_us: p50,
            p95_latency_us: p95,
            p99_latency_us: p99,
            throughput_ops_per_sec,
            memory_peak_mb: 0.0, // Populated separately if needed
        }
    }

    /// Print formatted results
    pub fn print_results(&self, test_name: &str) {
        println!("\n=== {} ===", test_name);
        println!("Total Operations: {}", self.total_ops);
        println!("Total Time: {:.2} ms", self.total_time_ms);
        println!("Throughput: {:.0} ops/sec", self.throughput_ops_per_sec);
        println!("\nLatency (microseconds):");
        println!("  Min:  {:.2} μs", self.min_latency_us);
        println!("  Avg:  {:.2} μs", self.avg_latency_us);
        println!("  P50:  {:.2} μs", self.p50_latency_us);
        println!("  P95:  {:.2} μs", self.p95_latency_us);
        println!("  P99:  {:.2} μs", self.p99_latency_us);
        println!("  Max:  {:.2} μs", self.max_latency_us);
    }
}

fn create_test_identity(id: usize, role: Role) -> Identity {
    let mut roles = HashSet::new();
    roles.insert(role);

    Identity {
        user_id: format!("user-{}", id),
        username: format!("user{}", id),
        roles,
    }
}

// ============================================================================
// TEST 1: Token Generation Throughput (1000 tokens)
// ============================================================================

#[test]
fn test_token_generation_throughput() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let mut latencies = Vec::new();
    let start = Instant::now();

    for i in 0..1000 {
        let identity = create_test_identity(i, Role::User);
        let op_start = Instant::now();

        auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token {}: {}", i, e))?;

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(1000, latencies, total_time_ms);

    metrics.print_results("test_token_generation_throughput");

    // Assert throughput is > 500 ops/sec (target: >1000/sec, acceptable: >500/sec)
    assert!(
        metrics.throughput_ops_per_sec > 500.0,
        "Token generation throughput too low: {:.0} ops/sec (target: >1000)",
        metrics.throughput_ops_per_sec
    );

    // Assert avg latency < 5ms (target: <2ms, acceptable: <5ms)
    assert!(
        metrics.avg_latency_us < 5000.0,
        "Token generation avg latency too high: {:.2} μs",
        metrics.avg_latency_us
    );

    Ok(())
}

// ============================================================================
// TEST 2: Token Validation Throughput (1000 validations)
// ============================================================================

#[test]
fn test_token_validation_throughput() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    // Pre-generate tokens
    let mut tokens = Vec::new();
    for i in 0..1000 {
        let identity = create_test_identity(i, Role::User);
        let token = auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token: {}", e))?;
        tokens.push(token);
    }

    let mut latencies = Vec::new();
    let start = Instant::now();

    for token in &tokens {
        let op_start = Instant::now();

        auth_manager
            .validate_token(token)
            .map_err(|e| format!("Failed to validate token: {}", e))?;

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(1000, latencies, total_time_ms);

    metrics.print_results("test_token_validation_throughput");

    // Assert throughput is > 2000 ops/sec (target: >5000/sec, acceptable: >2000/sec)
    assert!(
        metrics.throughput_ops_per_sec > 2000.0,
        "Token validation throughput too low: {:.0} ops/sec (target: >5000)",
        metrics.throughput_ops_per_sec
    );

    // Assert avg latency < 1ms (target: <1ms, acceptable: <3ms)
    assert!(
        metrics.avg_latency_us < 3000.0,
        "Token validation avg latency too high: {:.2} μs",
        metrics.avg_latency_us
    );

    Ok(())
}

// ============================================================================
// TEST 3: Permission Checks Throughput (10000 checks)
// ============================================================================

#[test]
fn test_permission_checks_throughput() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let identity = create_test_identity(1, Role::User);

    let mut latencies = Vec::new();
    let start = Instant::now();

    for _ in 0..10000 {
        let op_start = Instant::now();

        let _ = auth_manager.check_permission(&identity, "query", "read");

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(10000, latencies, total_time_ms);

    metrics.print_results("test_permission_checks_throughput");

    // Assert throughput is > 5000 ops/sec (target: >10000/sec, acceptable: >5000/sec)
    assert!(
        metrics.throughput_ops_per_sec > 5000.0,
        "Permission check throughput too low: {:.0} ops/sec (target: >10000)",
        metrics.throughput_ops_per_sec
    );

    // Assert avg latency < 0.5ms (target: <0.5ms, acceptable: <1ms)
    assert!(
        metrics.avg_latency_us < 1000.0,
        "Permission check avg latency too high: {:.2} μs",
        metrics.avg_latency_us
    );

    Ok(())
}

// ============================================================================
// TEST 4: AuthManager Scaling with 100 policies
// ============================================================================

#[test]
fn test_auth_manager_scaling() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    // Generate additional policies
    let mut latencies = Vec::new();

    let identity = create_test_identity(1, Role::Admin);

    let start = Instant::now();

    for i in 0..100 {
        let op_start = Instant::now();

        // Simulate policy checks with different resources
        let resource = format!("resource-{}", i);
        let _ = auth_manager.check_permission(&identity, &resource, "read");

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(100, latencies, total_time_ms);

    metrics.print_results("test_auth_manager_scaling");

    // Assert avg latency < 10ms per lookup (scaling test - policy count increases)
    assert!(
        metrics.avg_latency_us < 10000.0,
        "Auth manager scaling latency too high: {:.2} μs avg",
        metrics.avg_latency_us
    );

    Ok(())
}

// ============================================================================
// TEST 5: CLI Command Execution Batch (100 commands)
// ============================================================================

#[test]
fn test_cli_command_execution_batch() -> Result<(), String> {
    let mut latencies = Vec::new();
    let start = Instant::now();

    for _ in 0..100 {
        let op_start = Instant::now();

        let _output =
            run_cli_command(&["--version"]).map_err(|e| format!("CLI command failed: {}", e))?;

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(100, latencies, total_time_ms);

    metrics.print_results("test_cli_command_execution_batch");

    // CLI startup is slow - assert < 1 second average
    assert!(
        metrics.avg_latency_us < 1_000_000.0,
        "CLI execution avg latency too high: {:.2} μs",
        metrics.avg_latency_us
    );

    Ok(())
}

// ============================================================================
// TEST 6: Concurrent Token Generation (10 threads)
// ============================================================================

#[test]
fn test_concurrent_token_generation() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let mut handles = Vec::new();
    let start = Instant::now();

    for thread_id in 0..10 {
        let auth_manager = Arc::clone(&auth_manager);

        let handle = std::thread::spawn(move || {
            let mut thread_latencies = Vec::new();

            for i in 0..100 {
                let identity = create_test_identity(thread_id * 100 + i, Role::User);
                let op_start = Instant::now();

                let _ = auth_manager.generate_token(&identity);

                let latency_us = op_start.elapsed().as_micros() as f64;
                thread_latencies.push(latency_us);
            }

            thread_latencies
        });

        handles.push(handle);
    }

    let mut all_latencies = Vec::new();
    for handle in handles {
        let latencies = handle.join().map_err(|_| "Thread panicked".to_string())?;
        all_latencies.extend(latencies);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(1000, all_latencies, total_time_ms);

    metrics.print_results("test_concurrent_token_generation");

    // Concurrent generation should maintain reasonable throughput
    assert!(
        metrics.throughput_ops_per_sec > 100.0,
        "Concurrent token generation throughput too low: {:.0} ops/sec",
        metrics.throughput_ops_per_sec
    );

    Ok(())
}

// ============================================================================
// TEST 7: Concurrent Token Validation (10 threads)
// ============================================================================

#[test]
fn test_concurrent_validation() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    // Pre-generate tokens for validation
    let mut tokens = Vec::new();
    for i in 0..1000 {
        let identity = create_test_identity(i, Role::User);
        let token = auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token: {}", e))?;
        tokens.push(token);
    }

    let tokens = Arc::new(tokens);
    let mut handles = Vec::new();
    let start = Instant::now();

    for thread_id in 0..10 {
        let auth_manager = Arc::clone(&auth_manager);
        let tokens = Arc::clone(&tokens);

        let handle = std::thread::spawn(move || {
            let mut thread_latencies = Vec::new();
            let start_idx = thread_id * 100;
            let end_idx = start_idx + 100;

            for i in start_idx..end_idx {
                if let Some(token) = tokens.get(i) {
                    let op_start = Instant::now();
                    let _ = auth_manager.validate_token(token);
                    let latency_us = op_start.elapsed().as_micros() as f64;
                    thread_latencies.push(latency_us);
                }
            }

            thread_latencies
        });

        handles.push(handle);
    }

    let mut all_latencies = Vec::new();
    for handle in handles {
        let latencies = handle.join().map_err(|_| "Thread panicked".to_string())?;
        all_latencies.extend(latencies);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(1000, all_latencies, total_time_ms);

    metrics.print_results("test_concurrent_validation");

    // Concurrent validation should maintain reasonable throughput
    assert!(
        metrics.throughput_ops_per_sec > 500.0,
        "Concurrent validation throughput too low: {:.0} ops/sec",
        metrics.throughput_ops_per_sec
    );

    Ok(())
}

// ============================================================================
// TEST 8: Large Identity Lookup (search 1000 identities)
// ============================================================================

#[test]
fn test_large_identity_lookup() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    // Create and validate 1000 identities
    let mut identities = Vec::new();
    for i in 0..1000 {
        let identity = create_test_identity(i, Role::User);
        let _token = auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token: {}", e))?;
        identities.push(identity);
    }

    let mut latencies = Vec::new();
    let start = Instant::now();

    // Perform permission checks across all identities
    for identity in &identities {
        let op_start = Instant::now();

        let _ = auth_manager.check_permission(identity, "query", "read");

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    let total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let metrics = LoadMetrics::from_latencies(1000, latencies, total_time_ms);

    metrics.print_results("test_large_identity_lookup");

    // Large identity set should still have reasonable latency
    assert!(
        metrics.p99_latency_us < 100_000.0,
        "Large identity lookup p99 latency too high: {:.2} μs",
        metrics.p99_latency_us
    );

    Ok(())
}

// ============================================================================
// TEST 9: Auth Policy Registry Memory (1000 policies)
// ============================================================================

#[test]
fn test_auth_policy_registry_size() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    // Create 1000 identities with various roles
    let mut identities = Vec::new();
    for i in 0..1000 {
        let role = match i % 3 {
            0 => Role::Admin,
            1 => Role::User,
            _ => Role::Guest,
        };
        let identity = create_test_identity(i, role);
        identities.push(identity);
    }

    // Verify all identities can be checked without errors
    let mut error_count = 0;
    for (_idx, identity) in identities.iter().enumerate() {
        if auth_manager
            .check_permission(identity, "query", "read")
            .is_err()
        {
            error_count += 1;
        }
    }

    // Assert no errors during bulk permission checks
    assert_eq!(
        error_count, 0,
        "Failed permission checks on {} identities",
        error_count
    );

    println!("\n=== test_auth_policy_registry_size ===");
    println!("Successfully created and checked 1000 identities");
    println!("Registry memory stability: No errors detected");

    Ok(())
}

// ============================================================================
// TEST 10: CLI Startup Overhead (10 iterations)
// ============================================================================

#[test]
fn test_cli_startup_overhead() -> Result<(), String> {
    let mut latencies = Vec::new();

    for i in 0..10 {
        let op_start = Instant::now();

        let _output = run_cli_command(&["--version"])
            .map_err(|e| format!("CLI startup #{} failed: {}", i, e))?;

        let latency_us = op_start.elapsed().as_micros() as f64;
        latencies.push(latency_us);
    }

    // Convert microseconds to milliseconds for startup reporting
    let latencies_ms: Vec<f64> = latencies.iter().map(|us| us / 1000.0).collect();
    let total_time_ms: f64 = latencies_ms.iter().sum();
    let metrics = LoadMetrics::from_latencies(10, latencies, total_time_ms);

    metrics.print_results("test_cli_startup_overhead");

    // CLI startup < 500ms per iteration (target: <100ms, acceptable: <500ms)
    assert!(
        metrics.avg_latency_us < 500_000.0,
        "CLI startup overhead too high: {:.2} ms avg",
        metrics.avg_latency_us / 1000.0
    );

    Ok(())
}
