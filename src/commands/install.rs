use anyhow::Context as AnyhowContext;
use colored::Colorize;

use crate::args::{Args, InstallSubcommandArgs};
use crate::display;
use crate::installations::context::Context;
use crate::installations::{self};

pub async fn run(args: &Args, cmd: &InstallSubcommandArgs) -> anyhow::Result<()> {
    let ctx = Context::get_current(args.system)
        .await
        .context("Could not get current installation context.")?;

    let mut manifests = vec![];

    let bar = display::spinner("Installing packages...");
    for path in &cmd.path {
        bar.set_message(format!(
            "Installing package in {}...",
            path.display().to_string().blue()
        ));

        let manifest = installations::actions::install::install(ctx.clone(), path.clone())
            .await
            .context("Could not install package.")?;

        manifests.push(manifest);
    }

    bar.finish_and_clear();

    println!("Done! {} packages were installed:", manifests.len());

    for (index, manifest) in manifests.iter().enumerate() {
        println!(
            "{} {}",
            format!("{}.", index + 1).dimmed(),
            format!("{} v{}", manifest.package.name, manifest.package.version).blue()
        );
    }

    Ok(())
}
