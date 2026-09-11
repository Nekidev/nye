use axum::extract::FromRequestParts;

use crate::registries::http::server::database::{Token, User};
use crate::registries::http::server::state::RegistryState;
use crate::registries::http::server::v1::errors::{Error, OptionOrHttpError, ResultOrHttpError};
use crate::time;

pub struct Auth {
    pub user: User,
    pub token: Token,
}

impl FromRequestParts<RegistryState> for Auth {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &RegistryState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .or_http_401()?
            .to_str()
            .or_http_422()?;

        let (token_type, token_value) = header.split_once(" ").or_http_422()?;
        if !token_type.eq_ignore_ascii_case("bearer") {
            return Err(Error::http_422());
        }

        let mut db = state.db.connection().await.or_http_500()?;
        let token = toasty::query!(
            Token FILTER .id == #token_value AND .expires_at > #(time::utc_now_ms())
        )
        .first()
        .include(Token::fields().user())
        .exec(&mut db)
        .await
        .or_http_500()?;

        if let Some(token) = token {
            Ok(Auth {
                user: token.user.get().clone(),
                token,
            })
        } else {
            Err(Error::http_401())
        }
    }
}
