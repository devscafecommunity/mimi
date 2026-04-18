use crate::auth::Identity;
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::Ready;

pub struct AuthIdentity(pub Identity);

impl FromRequest for AuthIdentity {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(_req: &HttpRequest, _: &mut Payload) -> Self::Future {
        std::future::ready(Err(actix_web::error::ErrorUnauthorized(
            "Authentication required",
        )))
    }
}

pub struct OptionalAuthIdentity(pub Option<Identity>);

impl FromRequest for OptionalAuthIdentity {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(_req: &HttpRequest, _: &mut Payload) -> Self::Future {
        std::future::ready(Ok(OptionalAuthIdentity(None)))
    }
}
