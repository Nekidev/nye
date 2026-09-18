use std::collections::HashSet;

use anyhow::Context;
use colored::Colorize;

use crate::args::{Args, SigninSubcommandArgs};
use crate::display;
use crate::registries::config::RegistryConfig;
use crate::registries::{self, Registry, RegistryClient};

pub async fn run(_args: &Args, cmd: &SigninSubcommandArgs) -> anyhow::Result<()> {
    let mut registries = registries::config::load()
        .await
        .context("Could not load registries config in registries.toml.")?;

    let client = RegistryClient::from_url(cmd.registry.clone())
        .context("Could not get the registry client for the specified URL.")?;

    let spinner = display::spinner(format!(
        "Getting registry configuration for {}...",
        cmd.registry.to_string().blue()
    ));

    let config = client
        .get_config()
        .await
        .context("Could not get registry configuration.")?;

    spinner.finish_and_clear();

    if !config.is_signin_enabled {
        anyhow::bail!("This registry does not allow logging in.");
    }

    println!("Welcome back to {}!", config.name.blue());

    let duckity_task = config
        .duckity_signin_policy_id
        .as_ref()
        .map(|policy_id| tokio::spawn(duckity::solve(policy_id).into_future()));

    let username = {
        if let Some(value) = &cmd.username {
            value.clone()
        } else {
            tokio::task::spawn_blocking(move || {
                display::input("Username: ").context("Your input could not be read.")
            })
            .await
            .context("Reading the input panicked.")??
        }
    };
    let password = {
        if let Some(value) = &cmd.password {
            value.clone()
        } else {
            tokio::task::spawn_blocking(move || {
                display::password("Password: ").context("Your password could not be read.")
            })
            .await
            .context("Reading the input panicked.")??
        }
    };

    let duckity_token = {
        if let Some(task) = duckity_task {
            let spinner = display::spinner("Solving Duckity challenge, a sec...");

            let solution = task
                .await
                .context("The Duckity challenge-solving task panicked.")?
                .context("The Duckity challenge could not be fetched.")?;

            spinner.finish_and_clear();

            Some(solution)
        } else {
            None
        }
    };

    let spinner = display::spinner("Creating your account...");

    let credentials = client
        .signin(username, password, duckity_token)
        .await
        .context("Could not log into your account.")?;

    spinner.finish_and_clear();

    if registries.get_config(cmd.registry.to_string()).is_none() {
        // We're not doing anything async here so we can get away with no
        // tokio::task::spawn_blocking().
        let save = display::select_with_default_without_report(
            "Do you want to save this registry? ",
            ["Yes, please", "No"],
            0,
        )
        .context("Could not read your input.")?
            == 0;

        if save {
            let name = display::input_with_validation("Registry name: ", |name| {
                if registries.name_collides(name) {
                    Err("This registry name is already in use. Pick a different one.")
                } else {
                    Ok(())
                }
            })
            .context("Could not read your input.")?;

            let config = RegistryConfig {
                source: cmd.registry.clone(),
                aliases: HashSet::new(),
                version: 1,
            };

            registries.registries.insert(name, config);

            registries::config::save(&registries)
                .await
                .context("Could not store the registry in registries.toml.")?;
        }
    }

    println!("Done! You were logged into the registry as {}.", credentials.name);
    println!("Your credentials will be stored for up to 3 days of inactivity.");

    Ok(())
}
