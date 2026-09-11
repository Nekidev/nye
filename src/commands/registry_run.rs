use std::sync::Arc;

use anyhow::Context;
use colored::Colorize;

use crate::args::{Args, RegistrySubcommandRunSubcommandArgs};
use crate::registries;
use crate::registries::http::server::database;
use crate::registries::http::server::state::{DuckityState, RegistryState};

pub async fn run(_args: &Args, cmd: &RegistrySubcommandRunSubcommandArgs) -> anyhow::Result<()> {
    println!("Registry server is running at {}.", format!("http://{}/", cmd.bind).blue());

    let database = database::connect(cmd.database_url.to_string())
        .await
        .context("Could not connect to registry database.")?;
    let state = RegistryState {
        db: database,
        duckity: Arc::new(cmd.duckity.clone().map(|c| DuckityState {
            application_secret: c.application_secret,
            signin_protection_profile_id: c.signin_protection_profile_id,
            signup_protection_profile_id: c.signup_protection_profile_id,
        })),
    };

    registries::http::server::start(cmd.bind, state)
        .await
        .context("Could not run registry HTTP server.")?;

    Ok(())
}
