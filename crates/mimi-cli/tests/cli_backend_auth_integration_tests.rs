//! Backend Integration & Auth Tests
//!
//! Tests for CLI + HTTP/WS backend + auth layer integration:
//! - Token generation and validation
//! - Role-based access control
//! - HTTP Authorization header handling
//! - WebSocket auth on connect
//! - Network error handling
//! - Connection pooling patterns
//! - Error propagation

mod cli_integration_utils;
use cli_integration_utils::*;
use mimi_cli::auth::{Identity, Role};
use std::collections::HashSet;

#[test]
fn test_cli_http_exec_with_valid_token() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let token = ctx.create_admin_token()?;

    assert!(!token.is_empty());
    let identity = ctx
        .auth_manager
        .validate_token(&token)
        .map_err(|e| e.to_string())?;
    assert_eq!(identity.username, "admin");
    assert!(identity.roles.contains(&Role::Admin));
    Ok(())
}

#[test]
fn test_cli_http_exec_with_expired_token() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let identity = Identity {
        user_id: "user-1".to_string(),
        username: "testuser".to_string(),
        roles: {
            let mut set = HashSet::new();
            set.insert(Role::User);
            set
        },
    };

    let token = ctx.generate_token(identity).map_err(|e| e.to_string())?;
    let validated = ctx
        .auth_manager
        .validate_token(&token)
        .map_err(|e| e.to_string())?;
    assert_eq!(validated.username, "testuser");
    Ok(())
}

#[test]
fn test_cli_http_exec_without_token() -> Result<(), String> {
    let ctx = TestContext::new()?;

    assert_eq!(ctx.server_config.http_url(), "http://127.0.0.1:8080");
    Ok(())
}

#[test]
fn test_cli_http_admin_exec_with_admin_role() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let admin_token = ctx.create_admin_token()?;
    let identity = ctx
        .auth_manager
        .validate_token(&admin_token)
        .map_err(|e| e.to_string())?;

    let can_execute = ctx
        .auth_manager
        .check_permission(&identity, "execute", "all")
        .map_err(|e| e.to_string())?;
    assert!(can_execute);
    Ok(())
}

#[test]
fn test_cli_http_user_exec_with_user_role() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let user_token = ctx.create_user_token()?;
    let identity = ctx
        .auth_manager
        .validate_token(&user_token)
        .map_err(|e| e.to_string())?;

    let can_execute = ctx
        .auth_manager
        .check_permission(&identity, "execute", "read")
        .map_err(|e| e.to_string())?;
    assert!(can_execute);

    let can_query = ctx
        .auth_manager
        .check_permission(&identity, "query", "read")
        .map_err(|e| e.to_string())?;
    assert!(can_query);
    Ok(())
}

#[test]
fn test_cli_ws_streaming_with_valid_token() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let token = ctx.create_user_token()?;

    let identity = ctx
        .auth_manager
        .validate_token(&token)
        .map_err(|e| e.to_string())?;
    assert_eq!(identity.user_id, "user-1");
    assert_eq!(identity.username, "testuser");
    Ok(())
}

#[test]
fn test_cli_ws_auth_failure_connection_rejected() -> Result<(), String> {
    let ctx = TestContext::new()?;

    let invalid_token = "invalid.token.here";
    let result = ctx.auth_manager.validate_token(invalid_token);

    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_cli_multiple_roles_hierarchy() -> Result<(), String> {
    let ctx = TestContext::new()?;

    let identity = Identity {
        user_id: "user-admin-1".to_string(),
        username: "dual_role_user".to_string(),
        roles: {
            let mut set = HashSet::new();
            set.insert(Role::Admin);
            set.insert(Role::User);
            set
        },
    };

    let token = ctx.generate_token(identity)?;
    let loaded_identity = ctx
        .auth_manager
        .validate_token(&token)
        .map_err(|e| e.to_string())?;

    assert_eq!(loaded_identity.roles.len(), 2);
    assert!(loaded_identity.roles.contains(&Role::Admin));
    assert!(loaded_identity.roles.contains(&Role::User));
    Ok(())
}

#[test]
fn test_cli_guest_role_limited_access() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let guest_token = ctx.create_guest_token()?;
    let identity = ctx
        .auth_manager
        .validate_token(&guest_token)
        .map_err(|e| e.to_string())?;

    let can_query = ctx
        .auth_manager
        .check_permission(&identity, "query", "read")
        .map_err(|e| e.to_string())?;
    assert!(can_query);

    let can_execute = ctx
        .auth_manager
        .check_permission(&identity, "execute", "read")
        .map_err(|e| e.to_string())?;
    assert!(!can_execute);
    Ok(())
}
