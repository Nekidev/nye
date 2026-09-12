use std::sync::Arc;

use anyhow::Context;
use colored::Colorize;

use crate::args::{Args, RegistrySubcommandRunSubcommandArgs};
use crate::registries;
use crate::registries::http::server::database;
use crate::registries::http::server::state::{DuckityState, RegistryState};
use crate::registries::http::server::storage::StorageBackend;

pub async fn run(_args: &Args, cmd: &RegistrySubcommandRunSubcommandArgs) -> anyhow::Result<()> {
    let storage = get_storage(cmd).context("Could not initialize the storage backend.")?;

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
        storage: Arc::new(storage),
    };

    println!("Registry server is running at {}.", format!("http://{}/", cmd.bind).blue());

    registries::http::server::start(cmd.bind, state)
        .await
        .context("Could not run registry HTTP server.")?;

    Ok(())
}

fn get_storage(cmd: &RegistrySubcommandRunSubcommandArgs) -> anyhow::Result<StorageBackend> {
    if let Some(config) = &cmd.storage_s3 {
        return Ok(config.clone().into());
    }

    if let Some(config) = &cmd.storage_file {
        return Ok(config.clone().into());
    }

    anyhow::bail!(
        "No storage backend was configured. You need at least one to store package files."
    );
}
