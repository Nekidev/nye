use std::sync::Arc;

use anyhow::Context;
use colored::Colorize;

use crate::args::{Args, RegistrySubcommandRunSubcommandArgs};
use crate::registries;
use crate::registries::http::database;
use crate::registries::http::state::{DuckityState, RegistryConfig, RegistryState};
use crate::registries::http::storage::StorageBackend;

pub async fn run(_args: &Args, cmd: &RegistrySubcommandRunSubcommandArgs) -> anyhow::Result<()> {
    let storage = get_storage(cmd)
        .await
        .context("Could not initialize the storage backend.")?;

    let database = database::connect(cmd.database_url.to_string())
        .await
        .context("Could not connect to registry database.")?;
    let state = RegistryState {
        db: database,
        registry: Arc::new(RegistryConfig {
            name: cmd.registry.name.clone(),
            is_signin_enabled: !cmd.registry.no_signin,
            is_signup_enabled: !cmd.registry.no_signup,
        }),
        duckity: Arc::new(cmd.duckity.clone().map(|c| DuckityState {
            application_secret: c.application_secret,
            signin_policy_id: c.signin_policy_id,
            signup_policy_id: c.signup_policy_id,
        })),
        storage: Arc::new(storage),
    };

    println!("Registry server is running at {}.", format!("http://{}/", cmd.bind).blue());
    println!();

    registries::http::server::start(cmd.bind, state)
        .await
        .context("Could not run registry HTTP server.")?;

    Ok(())
}

async fn get_storage(cmd: &RegistrySubcommandRunSubcommandArgs) -> anyhow::Result<StorageBackend> {
    if let Some(config) = &cmd.storage_s3 {
        return StorageBackend::from_s3_args(config.clone()).await;
    }

    if let Some(config) = &cmd.storage_file {
        return StorageBackend::from_local_args(config.clone()).await;
    }

    anyhow::bail!(
        "No storage backend was configured. You need at least one to store package files."
    );
}
