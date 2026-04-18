use crate::auth::{AuthError, Identity, Policy, Role, TokenService};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct AuthManager {
    token_service: TokenService,
    policies: Arc<Mutex<HashMap<String, Policy>>>,
    identities: Arc<Mutex<HashMap<String, Identity>>>,
}

impl AuthManager {
    pub fn new(secret: String, token_ttl: i64) -> Self {
        AuthManager {
            token_service: TokenService::new(secret, token_ttl),
            policies: Arc::new(Mutex::new(HashMap::new())),
            identities: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_default_policies(&self) {
        let mut policies = self.policies.lock().unwrap();

        policies.insert(
            "admin_execute".to_string(),
            Policy {
                name: "admin_execute".to_string(),
                resource: "execute".to_string(),
                action: "all".to_string(),
                allowed_roles: {
                    let mut set = HashSet::new();
                    set.insert(Role::Admin);
                    set
                },
            },
        );

        policies.insert(
            "user_execute".to_string(),
            Policy {
                name: "user_execute".to_string(),
                resource: "execute".to_string(),
                action: "read".to_string(),
                allowed_roles: {
                    let mut set = HashSet::new();
                    set.insert(Role::User);
                    set.insert(Role::Admin);
                    set
                },
            },
        );

        policies.insert(
            "admin_query".to_string(),
            Policy {
                name: "admin_query".to_string(),
                resource: "query".to_string(),
                action: "read".to_string(),
                allowed_roles: {
                    let mut set = HashSet::new();
                    set.insert(Role::Admin);
                    set
                },
            },
        );

        policies.insert(
            "user_query".to_string(),
            Policy {
                name: "user_query".to_string(),
                resource: "query".to_string(),
                action: "read".to_string(),
                allowed_roles: {
                    let mut set = HashSet::new();
                    set.insert(Role::User);
                    set.insert(Role::Admin);
                    set
                },
            },
        );

        policies.insert(
            "guest_query".to_string(),
            Policy {
                name: "guest_query".to_string(),
                resource: "query".to_string(),
                action: "read".to_string(),
                allowed_roles: {
                    let mut set = HashSet::new();
                    set.insert(Role::Guest);
                    set
                },
            },
        );
    }

    pub fn generate_token(&self, identity: &Identity) -> Result<String, AuthError> {
        let auth_token = self.token_service.generate(identity)?;
        Ok(auth_token.token)
    }

    pub fn validate_token(&self, token: &str) -> Result<Identity, AuthError> {
        self.token_service.validate(token)
    }

    pub fn check_permission(
        &self,
        identity: &Identity,
        resource: &str,
        _action: &str,
    ) -> Result<bool, AuthError> {
        let policies = self.policies.lock().unwrap();

        for role in &identity.roles {
            let policy_key = format!("{}_{}", role.as_str(), resource);
            if let Some(policy) = policies.get(&policy_key) {
                if policy.allows(identity) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn register_identity(&self, identity: Identity) {
        let mut identities = self.identities.lock().unwrap();
        identities.insert(identity.user_id.clone(), identity);
    }

    pub fn get_identity(&self, user_id: &str) -> Result<Identity, AuthError> {
        let identities = self.identities.lock().unwrap();
        identities
            .get(user_id)
            .cloned()
            .ok_or(AuthError::UserNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_manager_lifecycle() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        manager.register_default_policies();

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

        let token = manager.generate_token(&identity);
        assert!(token.is_ok());

        let token_str = token.unwrap();
        let validated = manager.validate_token(&token_str);
        assert!(validated.is_ok());
    }

    #[test]
    fn test_permission_check() {
        let manager = AuthManager::new("secret".to_string(), 3600);
        manager.register_default_policies();

        let admin_identity = Identity {
            user_id: "admin1".to_string(),
            username: "admin".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::Admin);
                set
            },
        };

        let can_execute = manager.check_permission(&admin_identity, "execute", "all");
        assert!(can_execute.is_ok());
        assert!(can_execute.unwrap());
    }
}
