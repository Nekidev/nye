use anyhow::Context;
use colored::Colorize;
use nye_projects::{Project, TargetOrShared};
use nye_schemas::targets::Target;
use tokio::fs;

use crate::args::{Args, DevSubcommandInitSubcommandArgs};

pub async fn run(_args: &Args, cmd: &DevSubcommandInitSubcommandArgs) -> anyhow::Result<()> {
    if fs::try_exists(cmd.path.join("nye.toml"))
        .await
        .context("Could not check if project conflicted with an existing one.")?
    {
        anyhow::bail!(
            "There's already a nye project in {}. Choose a different path or delete the existing project.",
            cmd.path.display()
        );
    }

    let name = if let Some(name) = cmd.name.clone() {
        name
    } else {
        cmd.path.file_name().unwrap().to_string_lossy().to_string()
    };

    let project = Project::build(name)
        .context("The specified project name was invalid.")?
        .with_target(Target::get_current().context("Could not get current target.")?)
        .with_target(TargetOrShared::Shared)
        .init(cmd.path)
        .await
        .context("Could not initialize project.")?;

    let canonical_path = fs::canonicalize(&cmd.path)
        .await
        .context("Could not canonicalize project path for display.")?;

    println!(
        "Created project {} in {}.",
        project.manifest.package.name.blue(),
        canonical_path.display().to_string().blue()
    );

    Ok(())
}
