use crate::auth::{AuthError, AuthToken, Identity, Role, TokenClaims};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

pub struct TokenService {
    secret: String,
    token_ttl: i64,
}

impl TokenService {
    pub fn new(secret: String, token_ttl: i64) -> Self {
        TokenService { secret, token_ttl }
    }

    pub fn generate(&self, identity: &Identity) -> Result<AuthToken, AuthError> {
        let now = Utc::now().timestamp();
        let exp = now + self.token_ttl;

        let claims = TokenClaims {
            sub: identity.user_id.clone(),
            username: identity.username.clone(),
            roles: identity
                .roles
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
            iat: now,
            exp,
        };

        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());
        let token = encode(&Header::default(), &claims, &encoding_key)?;

        Ok(AuthToken {
            token,
            token_type: "Bearer".to_string(),
            expires_in: self.token_ttl,
            issued_at: now,
        })
    }

    pub fn validate(&self, token: &str) -> Result<Identity, AuthError> {
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());

        match decode::<TokenClaims>(token, &decoding_key, &Validation::default()) {
            Ok(data) => {
                let claims = data.claims;

                if claims.exp < Utc::now().timestamp() {
                    return Err(AuthError::TokenExpired);
                }

                let mut roles = std::collections::HashSet::new();
                for role_str in claims.roles {
                    if let Some(role) = Role::from_str(&role_str) {
                        roles.insert(role);
                    } else {
                        return Err(AuthError::RoleNotFound(role_str));
                    }
                }

                Ok(Identity {
                    user_id: claims.sub,
                    username: claims.username,
                    roles,
                })
            },
            Err(e) => Err(AuthError::JwtError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let service = TokenService::new("secret".to_string(), 3600);
        let identity = Identity {
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            roles: {
                let mut set = std::collections::HashSet::new();
                set.insert(Role::User);
                set
            },
        };

        let result = service.generate(&identity);
        assert!(result.is_ok());
        let auth_token = result.unwrap();
        assert_eq!(auth_token.token_type, "Bearer");
        assert_eq!(auth_token.expires_in, 3600);
    }

    #[test]
    fn test_validate_token() {
        let service = TokenService::new("secret".to_string(), 3600);
        let identity = Identity {
            user_id: "user1".to_string(),
            username: "testuser".to_string(),
            roles: {
                let mut set = std::collections::HashSet::new();
                set.insert(Role::User);
                set
            },
        };

        let token_result = service.generate(&identity);
        assert!(token_result.is_ok());

        let auth_token = token_result.unwrap();
        let validated = service.validate(&auth_token.token);
        assert!(validated.is_ok());

        let validated_identity = validated.unwrap();
        assert_eq!(validated_identity.user_id, "user1");
        assert_eq!(validated_identity.username, "testuser");
        assert!(validated_identity.roles.contains(&Role::User));
    }

    #[test]
    fn test_invalid_token() {
        let service = TokenService::new("secret".to_string(), 3600);
        let result = service.validate("invalid.token.here");
        assert!(result.is_err());
    }
}
