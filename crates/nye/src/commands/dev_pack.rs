use anyhow::Context as AnyhowContext;
use colored::Colorize;
use tokio::fs;

use crate::args::{Args, DevSubcommandPackSubcommandArgs};
use crate::display;
use crate::projects::TargetOrShared;
use crate::projects::actions::package;
use crate::projects::context::Context;

pub async fn run(_args: &Args, cmd: &DevSubcommandPackSubcommandArgs) -> anyhow::Result<()> {
    let ctx = Context::get_current()
        .await
        .context("Could not get current project context. Are you in a project's directory?")?;

    for target in &cmd.targets {
        if !ctx
            .manifest
            .targets
            .contains_key(&TargetOrShared::Target(*target))
        {
            anyhow::bail!(
                "The target `{target}` was passed to the pack command, but that target is not configured in the project's nye.toml manifest."
            );
        }

        if !target.is_supported() {
            anyhow::bail!("The target `{target}` is not supported by nye.");
        }
    }

    let mut targets = cmd.targets.clone();
    if targets.is_empty() {
        targets = ctx
            .manifest
            .targets
            .keys()
            .filter_map(|k| match k {
                TargetOrShared::Shared => None,
                TargetOrShared::Target(target) => Some(*target),
            })
            .collect();
    }
    let targets = targets;

    if !cmd.overwrite {
        for target in &targets {
            let path = ctx.get_dist_package_path(*target);
            let relative = pathdiff::diff_paths(&path, &ctx.path)
                .context("Could not get relative path of output file.")?;

            if fs::try_exists(path)
                .await
                .context("Could not check if output path already existed.")?
            {
                anyhow::bail!(
                    "There's already a package file at `{}`. To overwrite it, use `--overwrite` (`-o` for short).",
                    relative.display()
                );
            }
        }
    }

    for target in &targets {
        let bar = display::spinner(format!("Packaging for `{target}`..."));

        let result = package::package(&ctx, *target)
            .await
            .inspect_err(|_| {
                bar.abandon_with_message(format!(
                    "An error occurred while packaging for `{target}`."
                ))
            })
            .context("Could not package project for target.")?;

        let relative = pathdiff::diff_paths(result, &ctx.path)
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
