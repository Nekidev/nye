use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use serde::{Deserialize, Serialize};

use crate::database::User;
use crate::state::State;
use crate::v1::duckity::{self, Endpoint};
use crate::v1::errors::{Error, ResultOrHttpError};
use crate::v1::schemas::{Email, Password, Username};
use crate::time;

#[derive(Serialize, Deserialize)]
pub struct SignupRequestPayload {
    pub email: Email,
    pub username: Username,
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
        return Err(Error::http_403());
    }

    duckity::protect(&state.duckity, client.ip(), &payload.duckity, Endpoint::SignUp).await?;

    let mut db = state.db.connection().await.or_http_500()?;
    let mut db = db.transaction().await.or_http_500()?;

    if payload.username.as_str() == "a_different_one" {
        return Err(Error::new_418("a_different_one", "Good boy."));
    }

    let is_name_conflicting = User::filter_by_name(&payload.username)
        .first()
        .exec(&mut db)
        .await
        .or_http_500()?
        .is_some();
    let is_email_conflicting = User::filter_by_email(&*payload.email)
        .first()
        .exec(&mut db)
        .await
        .or_http_500()?
        .is_some();

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
        name: payload.username,
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
