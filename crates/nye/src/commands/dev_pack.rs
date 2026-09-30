use anyhow::Context as AnyhowContext;
use colored::Colorize;
use nye_projects::{Project, TargetOrShared};
use nye_schemas::targets::Target;

use crate::args::{Args, DevSubcommandPackSubcommandArgs};
use crate::display;

pub async fn run(_args: &Args, cmd: &DevSubcommandPackSubcommandArgs) -> anyhow::Result<()> {
    let project = Project::get_current()
        .await
        .context("Could not get current project context.")?
        .context("No project was found in the current directory nor any of its parents.")?;

    let targets = collect_targets(cmd, &project);
    validate_targets(cmd, &targets)?;

    for target in &targets {
        let bar = display::spinner(format!("Packaging for `{target}`..."));
        let result = project
            .package(*target)
            .await
            .inspect_err(|_| {
                bar.abandon_with_message(format!(
                    "An error occurred while packaging for `{target}`."
                ))
            })
            .context("Could not package project for target.")?;

        let relative = pathdiff::diff_paths(result, &project.path)
            .context("Could not get relative path of output file.")?;

        bar.finish_with_message(format!(
            "Packaged for {} at {}.",
            target.to_string().blue(),
            relative.display().to_string().blue()
        ));
    }

    let colored_targets: Vec<_> = targets.iter().map(|t| t.to_string().blue()).collect();

    println!();
    println!();
    println!(
        "Done! Packages for targets {} were placed in {}.",
        display::list(&colored_targets),
        "dist/".blue()
    );

    Ok(())
}

fn validate_targets(
    cmd: &DevSubcommandPackSubcommandArgs,
    targets: &[Target],
) -> anyhow::Result<()> {
    for target in &cmd.targets {
        if !targets.contains(target) {
            anyhow::bail!(
                "The target `{target}` was passed to the pack command, but that target is not configured in the project's nye.toml manifest."
            );
        }

        if !target.is_supported() {
            anyhow::bail!("The target `{target}` is not supported by nye.");
        }
    }

    Ok(())
}

fn collect_targets(cmd: &DevSubcommandPackSubcommandArgs, project: &Project) -> Vec<Target> {
    let mut targets = cmd.targets.clone();

    if targets.is_empty() {
        targets = project
            .manifest
            .targets
            .keys()
            .filter_map(|k| match k {
                TargetOrShared::Shared => None,
                TargetOrShared::Target(target) => Some(target),
            })
            .cloned()
            .collect();
    }

    targets
}
