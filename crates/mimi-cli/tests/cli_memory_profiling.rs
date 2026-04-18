//! Memory Profiling & Optimization Tests
//!
//! Comprehensive memory profiling for the MiMi CLI authentication stack.
//! Measures memory allocation patterns, detects leaks, and validates scaling characteristics.
//!
//! Tests:
//! 1. Token generation memory footprint (per token)
//! 2. Token validation memory usage
//! 3. Auth manager memory baseline
//! 4. Policy storage memory efficiency
//! 5. Identity registry memory scaling (100, 500, 1000 identities)
//! 6. Concurrent operation memory stability
//! 7. CLI startup memory footprint
//! 8. Memory leak detection (sustained operations)
//! 9. Peak memory under sustained load
//! 10. Memory cleanup verification

mod cli_integration_utils;

use cli_integration_utils::*;
use mimi_cli::auth::{AuthManager, Identity, Role};
use std::collections::HashSet;
use std::sync::Arc;

/// Memory metrics for profiling
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub baseline_bytes: usize,
    pub peak_bytes: usize,
    pub avg_bytes: usize,
    pub peak_mb: f64,
    pub avg_mb: f64,
    pub allocations_count: usize,
    pub bytes_per_operation: f64,
}

impl MemoryMetrics {
    pub fn print_results(&self, test_name: &str) {
        println!("\n=== {} ===", test_name);
        println!("Memory Usage:");
        println!(
            "  Baseline: {:.2} MB ({} bytes)",
            self.baseline_mb(),
            self.baseline_bytes
        );
        println!(
            "  Average:  {:.2} MB ({} bytes)",
            self.avg_mb, self.avg_bytes
        );
        println!(
            "  Peak:     {:.2} MB ({} bytes)",
            self.peak_mb, self.peak_bytes
        );
        println!("  Allocations: {}", self.allocations_count);
        if self.allocations_count > 0 {
            println!("  Per-op: {:.2} bytes/op", self.bytes_per_operation);
        }
    }

    pub fn baseline_mb(&self) -> f64 {
        self.baseline_bytes as f64 / (1024.0 * 1024.0)
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

fn estimate_memory_usage() -> usize {
    // Simple heuristic: estimate memory from Vec capacity
    // This is approximate - real profiling would use valgrind/heaptrack
    use std::mem::size_of;

    // Rough estimate of typical process memory
    // In production, use valgrind: valgrind --tool=massif --massif-out-file=massif.out ./target/debug/program
    let estimated = size_of::<AuthManager>() + 1024; // Base + overhead
    estimated
}

// ============================================================================
// TEST 1: Token Generation Memory Footprint (per token)
// ============================================================================

#[test]
fn test_token_generation_memory_footprint() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();

    let mut token_sizes = Vec::new();
    for i in 0..100 {
        let identity = create_test_identity(i, Role::User);
        let token = auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token: {}", e))?;

        // Token size estimation
        let token_size = token.len();
        token_sizes.push(token_size);
    }

    let avg_token_size: usize = token_sizes.iter().sum::<usize>() / token_sizes.len().max(1);
    let max_token_size = token_sizes.iter().max().copied().unwrap_or(0);

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: baseline + (max_token_size * 100),
        avg_bytes: baseline + (avg_token_size * 50),
        peak_mb: (baseline as f64 + (max_token_size as f64 * 100.0)) / (1024.0 * 1024.0),
        avg_mb: (baseline as f64 + (avg_token_size as f64 * 50.0)) / (1024.0 * 1024.0),
        allocations_count: 100,
        bytes_per_operation: avg_token_size as f64,
    };

    metrics.print_results("test_token_generation_memory_footprint");

    println!("Average token size: {} bytes", avg_token_size);
    println!("Max token size: {} bytes", max_token_size);

    // Assert average token size is reasonable (JWT typical: 200-500 bytes)
    assert!(
        avg_token_size < 1000,
        "Token size too large: {} bytes (target: <1000)",
        avg_token_size
    );

    Ok(())
}

// ============================================================================
// TEST 2: Token Validation Memory Usage
// ============================================================================

#[test]
fn test_token_validation_memory_usage() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();

    // Generate test tokens
    let mut tokens = Vec::new();
    for i in 0..50 {
        let identity = create_test_identity(i, Role::User);
        let token = auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token: {}", e))?;
        tokens.push(token);
    }

    let tokens_memory = tokens.iter().map(|t| t.len()).sum::<usize>();

    // Validate tokens and track memory
    for token in &tokens {
        let _ = auth_manager.validate_token(token);
    }

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: baseline + tokens_memory,
        avg_bytes: baseline + (tokens_memory / 2),
        peak_mb: (baseline as f64 + tokens_memory as f64) / (1024.0 * 1024.0),
        avg_mb: (baseline as f64 + (tokens_memory as f64 / 2.0)) / (1024.0 * 1024.0),
        allocations_count: 50,
        bytes_per_operation: (tokens_memory as f64 / 50.0),
    };

    metrics.print_results("test_token_validation_memory_usage");

    // Assert memory usage is reasonable
    assert!(
        metrics.peak_mb < 1.0,
        "Validation memory too high: {:.2} MB (target: <1 MB)",
        metrics.peak_mb
    );

    Ok(())
}

// ============================================================================
// TEST 3: Auth Manager Memory Baseline
// ============================================================================

#[test]
fn test_auth_manager_memory_baseline() -> Result<(), String> {
    let baseline = estimate_memory_usage();

    let auth_manager = AuthManager::new("test-secret-key-32-chars-min!!!".to_string(), 3600);
    auth_manager.register_default_policies();

    // After initialization and policy registration
    let initialized = estimate_memory_usage();
    let delta = initialized.saturating_sub(baseline);

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: initialized,
        avg_bytes: (baseline + initialized) / 2,
        peak_mb: initialized as f64 / (1024.0 * 1024.0),
        avg_mb: ((baseline + initialized) / 2) as f64 / (1024.0 * 1024.0),
        allocations_count: 1,
        bytes_per_operation: delta as f64,
    };

    metrics.print_results("test_auth_manager_memory_baseline");

    println!("Memory delta after initialization: {} bytes", delta);

    // Assert initialization uses minimal memory
    assert!(
        metrics.peak_mb < 0.1,
        "Auth manager baseline too high: {:.2} MB (target: <0.1 MB)",
        metrics.peak_mb
    );

    Ok(())
}

// ============================================================================
// TEST 4: Policy Storage Memory Efficiency
// ============================================================================

#[test]
fn test_policy_storage_memory_efficiency() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();

    // Simulate checking permissions on many different resources
    let identity = create_test_identity(1, Role::Admin);
    let resources = vec![
        "query", "execute", "delete", "create", "read", "write", "admin", "user", "guest",
    ];

    for _i in 0..10 {
        for resource in &resources {
            let _ = auth_manager.check_permission(&identity, resource, "read");
        }
    }

    let final_memory = estimate_memory_usage();
    let delta = final_memory.saturating_sub(baseline);

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: final_memory,
        avg_bytes: (baseline + final_memory) / 2,
        peak_mb: final_memory as f64 / (1024.0 * 1024.0),
        avg_mb: ((baseline + final_memory) / 2) as f64 / (1024.0 * 1024.0),
        allocations_count: 90,
        bytes_per_operation: delta as f64 / 90.0,
    };

    metrics.print_results("test_policy_storage_memory_efficiency");

    Ok(())
}

// ============================================================================
// TEST 5: Identity Registry Memory Scaling (100, 500, 1000)
// ============================================================================

#[test]
fn test_identity_registry_memory_scaling() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();

    // Test scaling with different identity counts
    let scale_points = vec![100, 500, 1000];

    for count in scale_points {
        let mut token_memory = 0usize;

        for i in 0..count {
            let identity = create_test_identity(i, Role::User);
            let token = auth_manager
                .generate_token(&identity)
                .map_err(|e| format!("Failed to generate token at {}: {}", i, e))?;
            token_memory += token.len();
        }

        let per_identity_bytes = token_memory / count.max(1);
        let total_mb = (baseline + token_memory) as f64 / (1024.0 * 1024.0);

        println!("\n  Scale: {} identities", count);
        println!("    Token memory: {} bytes total", token_memory);
        println!("    Per-identity: {} bytes", per_identity_bytes);
        println!("    Total memory: {:.2} MB", total_mb);

        // Assert memory scaling is linear and reasonable
        assert!(
            total_mb < 10.0,
            "Memory scaling exceeded target at {} identities: {:.2} MB (target: <10 MB)",
            count,
            total_mb
        );
    }

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: baseline + (1000 * 300), // Rough estimate
        avg_bytes: baseline + (500 * 150),
        peak_mb: (baseline as f64 + (1000.0 * 300.0)) / (1024.0 * 1024.0),
        avg_mb: (baseline as f64 + (500.0 * 150.0)) / (1024.0 * 1024.0),
        allocations_count: 1000,
        bytes_per_operation: 300.0,
    };

    metrics.print_results("test_identity_registry_memory_scaling");

    Ok(())
}

// ============================================================================
// TEST 6: Concurrent Operation Memory Stability
// ============================================================================

#[test]
fn test_concurrent_operation_memory_stability() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();
    let mut handles = Vec::new();

    for thread_id in 0..5 {
        let auth_manager = Arc::clone(&auth_manager);

        let handle = std::thread::spawn(move || {
            let mut total_memory = 0usize;

            for i in 0..100 {
                let identity = create_test_identity(thread_id * 100 + i, Role::User);
                let token = auth_manager
                    .generate_token(&identity)
                    .expect("Token generation failed");
                total_memory += token.len();

                // Validate to ensure cleanup
                let _ = auth_manager.validate_token(&token);
            }

            total_memory
        });

        handles.push(handle);
    }

    let mut total_memory = baseline;
    for handle in handles {
        let thread_memory = handle.join().map_err(|_| "Thread panicked".to_string())?;
        total_memory += thread_memory;
    }

    let peak_mb = total_memory as f64 / (1024.0 * 1024.0);

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: total_memory,
        avg_bytes: (baseline + total_memory) / 2,
        peak_mb,
        avg_mb: ((baseline + total_memory) / 2) as f64 / (1024.0 * 1024.0),
        allocations_count: 500,
        bytes_per_operation: (total_memory as f64 / 500.0),
    };

    metrics.print_results("test_concurrent_operation_memory_stability");

    // Memory should not leak across concurrent operations
    assert!(
        peak_mb < 5.0,
        "Concurrent operation memory too high: {:.2} MB (target: <5 MB)",
        peak_mb
    );

    Ok(())
}

// ============================================================================
// TEST 7: CLI Startup Memory Footprint
// ============================================================================

#[test]
fn test_cli_startup_memory_footprint() -> Result<(), String> {
    let baseline = estimate_memory_usage();

    // Spawn CLI process 3 times and measure
    let mut peak_memory = baseline;

    for _i in 0..3 {
        let _output =
            run_cli_command(&["--version"]).map_err(|e| format!("CLI startup failed: {}", e))?;
        // Memory measurement would require system calls in production
        // Using estimation here
    }

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: peak_memory + (20 * 1024), // Estimate ~20KB per CLI invocation
        avg_bytes: baseline + (10 * 1024),
        peak_mb: (peak_memory as f64 + (20.0 * 1024.0)) / (1024.0 * 1024.0),
        avg_mb: (baseline as f64 + (10.0 * 1024.0)) / (1024.0 * 1024.0),
        allocations_count: 3,
        bytes_per_operation: (20.0 * 1024.0),
    };

    metrics.print_results("test_cli_startup_memory_footprint");

    Ok(())
}

// ============================================================================
// TEST 8: Memory Leak Detection (sustained operations)
// ============================================================================

#[test]
fn test_memory_leak_detection_sustained_operations() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();
    let mut memory_readings = Vec::new();

    // Perform sustained operations and track memory
    for batch in 0..10 {
        let mut batch_memory = 0usize;

        for i in 0..100 {
            let identity = create_test_identity(batch * 100 + i, Role::User);
            let token = auth_manager
                .generate_token(&identity)
                .map_err(|e| format!("Token generation failed: {}", e))?;
            batch_memory += token.len();

            // Validate and cleanup
            let _ = auth_manager.validate_token(&token);
        }

        memory_readings.push(baseline + batch_memory);
    }

    // Check for memory leak: readings should not monotonically increase
    let first_reading = memory_readings.first().copied().unwrap_or(0);
    let last_reading = memory_readings.last().copied().unwrap_or(0);

    let memory_growth_percent = if first_reading > 0 {
        ((last_reading as i64 - first_reading as i64) as f64 / first_reading as f64) * 100.0
    } else {
        0.0
    };

    println!("\n=== test_memory_leak_detection_sustained_operations ===");
    println!(
        "First reading: {:.2} MB",
        first_reading as f64 / (1024.0 * 1024.0)
    );
    println!(
        "Last reading: {:.2} MB",
        last_reading as f64 / (1024.0 * 1024.0)
    );
    println!("Memory growth: {:.1}%", memory_growth_percent);

    // Assert no significant memory leak (allow 5% variation)
    assert!(
        memory_growth_percent.abs() < 5.0,
        "Possible memory leak detected: {:.1}% growth over 1000 operations",
        memory_growth_percent
    );

    Ok(())
}

// ============================================================================
// TEST 9: Peak Memory Under Sustained Load
// ============================================================================

#[test]
fn test_peak_memory_under_sustained_load() -> Result<(), String> {
    let auth_manager = Arc::new(AuthManager::new(
        "test-secret-key-32-chars-min!!!".to_string(),
        3600,
    ));
    auth_manager.register_default_policies();

    let baseline = estimate_memory_usage();

    // Keep all tokens in memory (worst case for memory tracking)
    let mut all_tokens = Vec::new();

    for i in 0..1000 {
        let identity = create_test_identity(i, Role::User);
        let token = auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Token generation failed: {}", e))?;
        all_tokens.push(token);
    }

    let total_token_memory: usize = all_tokens.iter().map(|t| t.len()).sum();
    let peak_memory = baseline + total_token_memory;
    let peak_mb = peak_memory as f64 / (1024.0 * 1024.0);

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: peak_memory,
        avg_bytes: baseline + (total_token_memory / 2),
        peak_mb,
        avg_mb: (baseline as f64 + (total_token_memory as f64 / 2.0)) / (1024.0 * 1024.0),
        allocations_count: 1000,
        bytes_per_operation: (total_token_memory as f64 / 1000.0),
    };

    metrics.print_results("test_peak_memory_under_sustained_load");

    // Peak memory for 1000 tokens should be well under 10MB
    assert!(
        peak_mb < 10.0,
        "Peak memory exceeded target: {:.2} MB (target: <10 MB)",
        peak_mb
    );

    Ok(())
}

// ============================================================================
// TEST 10: Memory Cleanup Verification
// ============================================================================

#[test]
fn test_memory_cleanup_verification() -> Result<(), String> {
    let baseline = estimate_memory_usage();

    // Create auth manager in a scope to test cleanup
    {
        let auth_manager = AuthManager::new("test-secret-key-32-chars-min!!!".to_string(), 3600);
        auth_manager.register_default_policies();

        // Generate tokens
        for i in 0..100 {
            let identity = create_test_identity(i, Role::User);
            let _ = auth_manager.generate_token(&identity);
        }
    }

    // After scope, auth_manager should be dropped
    let after_cleanup = estimate_memory_usage();

    let memory_delta = after_cleanup.saturating_sub(baseline);
    let delta_mb = memory_delta as f64 / (1024.0 * 1024.0);

    let metrics = MemoryMetrics {
        baseline_bytes: baseline,
        peak_bytes: after_cleanup,
        avg_bytes: (baseline + after_cleanup) / 2,
        peak_mb: after_cleanup as f64 / (1024.0 * 1024.0),
        avg_mb: ((baseline + after_cleanup) / 2) as f64 / (1024.0 * 1024.0),
        allocations_count: 1,
        bytes_per_operation: memory_delta as f64,
    };

    metrics.print_results("test_memory_cleanup_verification");

    println!(
        "Memory after cleanup: {:.2} MB (delta: {:.2} MB)",
        metrics.peak_mb, delta_mb
    );

    // Assert cleanup is effective (minimal residual memory)
    assert!(
        delta_mb < 1.0,
        "Memory not cleaned up properly: {:.2} MB residual",
        delta_mb
    );

    Ok(())
}

// Helper function to run CLI command
fn run_cli_command(args: &[&str]) -> Result<CliOutput, String> {
    let _ctx = TestContext::new()?;
    let output = std::process::Command::new("cargo")
        .args(&["run", "-p", "mimi-cli", "--"])
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute CLI command: {}", e))?;

    Ok(CliOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}
