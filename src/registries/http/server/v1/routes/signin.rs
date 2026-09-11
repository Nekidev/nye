use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::database::{Token, TokenKind, User};
use crate::registries::http::server::state::State;
use crate::registries::http::server::v1::duckity::{self, Endpoint};
use crate::registries::http::server::v1::errors::{Error, ResultOrHttpError};
use crate::registries::http::server::v1::schemas::{Login, Password};
use crate::time;

#[derive(Serialize, Deserialize)]
pub struct SigninRequestPayload {
    pub login: Login,
    pub password: Password,

    #[serde(default)]
    pub duckity: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SigninResponsePayload {
    pub access_token: String,
    pub access_token_expires_in: u64,
    pub refresh_token: String,
    pub refresh_token_expires_in: u64,
}

pub async fn handle(
    state: State,
    client: ConnectInfo<SocketAddr>,
    Json(payload): Json<SigninRequestPayload>,
) -> Result<Json<SigninResponsePayload>, Error> {
    duckity::protect(&state.duckity, client.ip(), &payload.duckity, Endpoint::SignIn).await?;

    let mut db = state.db.connection().await.or_http_500()?;
    let mut db = db.transaction().await.or_http_500()?;

    let query = match &payload.login {
        Login::Email(email) => User::filter_by_email(email),
        Login::Username(username) => User::filter_by_name(username),
    };

    let user = query.first().exec(&mut db).await.or_http_500()?;

    if let Some(user) = user
        && payload.password.verify(&user.password).await
    {
        let access_token = toasty::create!(Token {
            id: nanoid::nanoid!(),
            kind: TokenKind::Access,
            user_id: &user.id,
            created_at: time::utc_now_ms(),
            updated_at: time::utc_now_ms(),
            expires_at: time::utc_now_ms() + 60 * 60 * 1000,
        })
        .exec(&mut db)
        .await
        .or_http_500()?;
        let refresh_token = toasty::create!(Token {
            id: nanoid::nanoid!(),
            kind: TokenKind::Refresh,
            user_id: &user.id,
            created_at: time::utc_now_ms(),
            updated_at: time::utc_now_ms(),
            expires_at: time::utc_now_ms() + 7 * 24 * 60 * 60 * 1000,
        })
        .exec(&mut db)
        .await
        .or_http_500()?;

        db.commit().await.or_http_500()?;

        Ok(SigninResponsePayload {
            access_token: access_token.id,
            access_token_expires_in: 3600,
            refresh_token: refresh_token.id,
            refresh_token_expires_in: 3600 * 24 * 7,
        }
        .into())
    } else {
        Err(Error::new_401(
            "Invalid Credentials",
            "The username, email, or password you provided was incorrect. Revise your inputs and try again.",
        ))
    }
}
