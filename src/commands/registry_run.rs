use anyhow::Context;
use colored::Colorize;

use crate::args::{Args, RegistrySubcommandRunSubcommandArgs};
use crate::registries;

pub async fn run(_args: &Args, cmd: &RegistrySubcommandRunSubcommandArgs) -> anyhow::Result<()> {
    println!(
        "Registry server is running at {}.",
        format!("http://{}/", cmd.bind).blue()
    );

    registries::http::server::start(cmd.bind)
        .await
        .context("Could not run registry HTTP server.")?;

    Ok(())
}
