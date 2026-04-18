use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// User identity extracted from token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub user_id: String,
    pub username: String,
    pub roles: HashSet<Role>,
}

/// Authorization role
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
    Guest,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
            Role::Guest => "guest",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Role::Admin),
            "user" => Some(Role::User),
            "guest" => Some(Role::Guest),
            _ => None,
        }
    }
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String, // subject (user_id)
    pub username: String,
    pub roles: Vec<String>,
    pub iat: i64, // issued at
    pub exp: i64, // expiration
}

/// Authentication token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub token_type: String, // "Bearer"
    pub expires_in: i64,    // seconds until expiration
    pub issued_at: i64,     // unix timestamp
}

/// RBAC policy
#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub resource: String, // e.g., "execute", "query", "config"
    pub action: String,   // e.g., "read", "write", "delete"
    pub allowed_roles: HashSet<Role>,
}

impl Policy {
    pub fn allows(&self, identity: &Identity) -> bool {
        identity
            .roles
            .iter()
            .any(|role| self.allowed_roles.contains(role))
    }
}
