use std::str::FromStr;

use anyhow::Context;
use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::registries::http::v1::routes::config::Config;
use crate::registries::http::v1::routes::signin::{SigninRequestPayload, SigninResponsePayload};
use crate::registries::http::v1::routes::signup::{SignupRequestPayload, SignupResponsePayload};
use crate::registries::http::v1::schemas::{Email, ErrorSchema, Login, Password, Username};
use crate::registries::{Credentials, Registry, RegistryConfig, config};

pub struct HttpRegistryClient {
    base_url: Url,
}

impl HttpRegistryClient {
    pub fn new(config: &config::RegistryConfig) -> Self {
        Self {
            base_url: config.source.clone(),
        }
    }

    pub fn from_url(url: Url) -> anyhow::Result<Self> {
        if !["http", "https"].contains(&url.scheme()) {
            anyhow::bail!(concat!(
                "HTTP(S) registries require their URL to use the http/https scheme. This one did ",
                "not."
            ));
        }

        Ok(Self { base_url: url })
    }
}

impl Registry for HttpRegistryClient {
    async fn get_config(&self) -> anyhow::Result<RegistryConfig> {
        let response: Config = get(self.base_url.join("config").unwrap()).await?;

        Ok(RegistryConfig {
            name: response.name,
            is_signin_enabled: response.is_signin_enabled,
            is_signup_enabled: response.is_signup_enabled,
            duckity_signin_policy_id: response.duckity_signin_policy_id,
            duckity_signup_policy_id: response.duckity_signup_policy_id,
        })
    }

    async fn signup(
        &self,
        email: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<()> {
        let _: SignupResponsePayload = post(
            self.base_url.join("signup").unwrap(),
            SignupRequestPayload {
                email: Email::from_str(&email.into()).context("The email provided was invalid.")?,
                username: Username::from_str(&username.into())
                    .context("The username provided was invalid.")?,
                password: Password::from_str(&password.into())
                    .context("The password provided was invalid.")?,
                duckity: duckity.map(Into::into),
            },
        )
        .await?;

        Ok(())
    }

    async fn signin(
        &self,
        username: impl Into<String>,
        password: impl Into<String>,
        duckity: Option<impl Into<String>>,
    ) -> anyhow::Result<Credentials> {
        let response: SigninResponsePayload = post(
            self.base_url.join("signin").unwrap(),
            SigninRequestPayload {
                login: Login::from_str(&username.into())
                    .context("The username provided was invalid.")?,
                password: Password::from_str(&password.into())
                    .context("The password provided was invalid.")?,
                duckity: duckity.map(Into::into),
            },
        )
        .await?;

        Ok(Credentials {
            name: response.user_name,
            access_token: response.access_token,
            access_token_expires_in: response.access_token_expires_in,
            refresh_token: response.refresh_token,
            refresh_token_expires_in: response.refresh_token_expires_in,
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Response<T, E> {
    Ok(T),
    Err(E),
}

pub async fn get<T>(url: Url) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let client = reqwest::Client::new();

    let request = client
        .get(url)
        .header(USER_AGENT, format!("nye/{}", env!("CARGO_PKG_VERSION")));
    let response = request
        .send()
        .await
        .context("Could not make a GET request to the registry.")?;
    let response: Response<T, ErrorSchema> = response
        .json()
        .await
        .context("Could not deserialize the registry's response into the specified JSON schema.")?;

    match response {
        Response::Ok(response) => Ok(response),
        Response::Err(error) => anyhow::bail!("{}: {}", error.title, error.message),
    }
}

pub async fn post<B, T>(url: Url, body: B) -> anyhow::Result<T>
where
    B: Serialize,
    T: DeserializeOwned,
{
    let client = reqwest::Client::new();

    let request = client
        .post(url)
        .json(&body)
        .header(USER_AGENT, format!("nye/{}", env!("CARGO_PKG_VERSION")));
    let response = request
        .send()
        .await
        .context("Could not make a POST request to the registry.")?;
    let response: Response<T, ErrorSchema> = response
        .json()
        .await
        .context("Could not deserialize the registry's response into the specified JSON schema.")?;

    match response {
        Response::Ok(response) => Ok(response),
        Response::Err(error) => anyhow::bail!("{}: {}", error.title, error.message),
    }
}
