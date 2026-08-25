use anyhow::Context;
use colored::Colorize;
use tokio::fs;

use crate::args::{Args, DevSubcommandInitSubcommandArgs};
use crate::projects::actions::create;

pub async fn run(_args: &Args, cmd: &DevSubcommandInitSubcommandArgs) -> anyhow::Result<()> {
    let manifest = create::create(cmd.path.clone(), cmd.name.clone())
        .await
        .context("Could not create project.")?;

    let canonical_path = fs::canonicalize(&cmd.path)
        .await
        .context("Could not canonicalize project path for display.")?;

    println!(
        "Created project {} in {}.",
        manifest.package.name.blue(),
        canonical_path.display().to_string().blue()
    );

    Ok(())
}
