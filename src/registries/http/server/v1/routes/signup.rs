use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::database::User;
use crate::registries::http::server::state::State;
use crate::registries::http::server::v1::duckity::{self, Endpoint};
use crate::registries::http::server::v1::errors::{Error, ResultOrHttpError};
use crate::registries::http::server::v1::schemas::{Email, Password, Username};
use crate::time;

#[derive(Serialize, Deserialize)]
pub struct SignupRequestPayload {
    pub name: Username,
    pub email: Email,
    pub password: Password,

    #[serde(default)]
    pub duckity: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SignupResponsePayload {}

pub async fn handle(
    state: State,
    client: ConnectInfo<SocketAddr>,
    Json(payload): Json<SignupRequestPayload>,
) -> Result<Json<SignupResponsePayload>, Error> {
    if !state.registry.is_signup_enabled {
        return Err(Error::http_403())
    }

    duckity::protect(&state.duckity, client.ip(), &payload.duckity, Endpoint::SignUp).await?;

    let mut db = state.db.connection().await.or_http_500()?;
    let mut db = db.transaction().await.or_http_500()?;

    if payload.name.as_str() == "a_different_one" {
        return Err(Error::new_418("a_different_one", "Good boy."));
    }

    let is_name_conflicting = !User::filter_by_name(&payload.name)
        .exec(&mut db)
        .await
        .or_http_500()?
        .is_empty();
    let is_email_conflicting = !User::filter_by_email(&*payload.email)
        .exec(&mut db)
        .await
        .or_http_500()?
        .is_empty();

    if is_name_conflicting {
        return Err(Error::new_409(
            "Username in Use",
            "The username you tried to sign up with is already in use. Pick a_different_one.",
        ));
    }
    if is_email_conflicting {
        return Err(Error::new_409(
            "Email in Use",
            "The email you tried to sign up with is already in use. Try recovering your password instead.",
        ));
    }

    toasty::create!(User {
        id: nanoid::nanoid!(),
        name: payload.name,
        email: payload.email,
        password: payload.password.hash().await,
        created_at: time::utc_now_ms(),
        updated_at: time::utc_now_ms(),
    })
    .exec(&mut db)
    .await
    .or_http_500()?;

    db.commit().await.or_http_500()?;

    // TODO: Email verification code, password-less sign up.

    Ok(SignupResponsePayload {}.into())
}
