#[cfg(test)]
mod auth_integration_tests {
    use mimi_cli::auth::{AuthManager, Identity, Role};
    use std::collections::HashSet;

    #[test]
    fn test_token_generation_and_validation() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        let identity = Identity {
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::User);
                set
            },
        };

        let token = manager.generate_token(&identity);
        assert!(token.is_ok());

        let validated = manager.validate_token(&token.unwrap());
        assert!(validated.is_ok());
    }

    #[test]
    fn test_invalid_token_rejection() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        let result = manager.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_role_based_permissions() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        manager.register_default_policies();

        let admin = Identity {
            user_id: "admin1".to_string(),
            username: "admin".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::Admin);
                set
            },
        };

        let user = Identity {
            user_id: "user1".to_string(),
            username: "user".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::User);
                set
            },
        };

        let guest = Identity {
            user_id: "guest1".to_string(),
            username: "guest".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::Guest);
                set
            },
        };

        assert!(manager.check_permission(&admin, "execute", "all").unwrap());
        assert!(manager.check_permission(&user, "execute", "read").unwrap());
        assert!(manager.check_permission(&guest, "query", "read").unwrap());
    }

    #[test]
    fn test_multiple_roles_hierarchy() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        manager.register_default_policies();

        let mut roles = HashSet::new();
        roles.insert(Role::Admin);
        roles.insert(Role::User);

        let multi_role = Identity {
            user_id: "power_user".to_string(),
            username: "poweruser".to_string(),
            roles,
        };

        assert!(manager
            .check_permission(&multi_role, "execute", "all")
            .unwrap());
        assert!(manager
            .check_permission(&multi_role, "query", "read")
            .unwrap());
    }

    #[test]
    fn test_identity_registration() {
        let manager = AuthManager::new("secret".to_string(), 3600);

        let identity = Identity {
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::User);
                set
            },
        };

        manager.register_identity(identity.clone());
        let retrieved = manager.get_identity("user1");
        assert!(retrieved.is_ok());

        let retrieved_id = retrieved.unwrap();
        assert_eq!(retrieved_id.user_id, "user1");
        assert_eq!(retrieved_id.username, "testuser");
    }

    #[test]
    fn test_token_expiration() {
        let manager = AuthManager::new("secret".to_string(), 1);
        let identity = Identity {
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::User);
                set
            },
        };

        let token = manager.generate_token(&identity);
        assert!(token.is_ok());

        std::thread::sleep(std::time::Duration::from_secs(2));

        let validated = manager.validate_token(&token.unwrap());
        assert!(validated.is_err());
    }

    #[test]
    fn test_access_denied_for_insufficient_roles() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        manager.register_default_policies();

        let guest = Identity {
            user_id: "guest1".to_string(),
            username: "guest".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::Guest);
                set
            },
        };

        let result = manager.check_permission(&guest, "execute", "write");
        match result {
            Ok(allowed) => assert!(!allowed),
            Err(_) => {},
        }
    }
}
