use std::net::SocketAddr;
use std::time::Duration;

use axum::Json;
use axum::extract::ConnectInfo;
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::database::{Token, TokenKind, User};
use crate::registries::http::server::state::State;
use crate::registries::http::server::v1::errors::{Error, OrHttpError};
use crate::registries::http::server::v1::schemas::{Login, Password};

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
    // TODO: Migrate this check and /v1/signup's to a duckity.rs module, alternate between
    //       protection profiles in config with an enum to keep it DRY.

    if let Some(config) = &*state.duckity {
        let Some(solution) = payload.duckity else {
            return Err(Error::new_422(
                "Missing Duckity Solution",
                "This registry requires Duckity solution tokens to be sent when signing in.",
            ));
        };

        let is_valid = duckity::validate(
            solution,
            client.ip(),
            &config.application_secret,
            &config.signin_protection_profile_id,
        )
        .await
        .or_http_500()?;

        if !is_valid {
            return Err(Error::new_422(
                "Invalid Duckity Solution",
                "The Duckity solution token provided was invalid.",
            ));
        }
    }

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
            created_at: Zoned::now(),
            updated_at: Zoned::now(),
            expires_at: Zoned::now() + Duration::from_hours(1),
        })
        .exec(&mut db)
        .await
        .or_http_500()?;
        let refresh_token = toasty::create!(Token {
            id: nanoid::nanoid!(),
            kind: TokenKind::Refresh,
            user_id: &user.id,
            created_at: Zoned::now(),
            updated_at: Zoned::now(),
            expires_at: Zoned::now() + Duration::from_hours(24) * 7,
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
