use std::net::IpAddr;

use crate::registries::http::server::state::DuckityState;
use crate::registries::http::server::v1::errors::{Error, ResultOrHttpError};

#[derive(Debug, Clone, Copy)]
pub enum Endpoint {
    SignIn,
    SignUp,
}

impl Endpoint {
    fn protection_profile_id<'a>(&self, state: &'a DuckityState) -> &'a str {
        match &self {
            Endpoint::SignIn => &state.signin_protection_profile_id,
            Endpoint::SignUp => &state.signup_protection_profile_id,
        }
    }
}

pub async fn protect(
    state: &Option<DuckityState>,
    client: IpAddr,
    solution: &Option<String>,
    endpoint: Endpoint,
) -> Result<(), Error> {
    if let Some(config) = &state {
        let Some(solution) = solution else {
            return Err(Error::new_422(
                "Missing Duckity Solution",
                "This registry requires Duckity solution tokens to be sent when using this endpoint.",
            ));
        };

        let is_valid = duckity::validate(
            solution,
            client,
            &config.application_secret,
            endpoint.protection_profile_id(config),
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

    Ok(())
}
