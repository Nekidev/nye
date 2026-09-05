use anyhow::Context as AnyhowContext;
use colored::Colorize;

use crate::args::{Args, UninstallSubcommandArgs};
use crate::display;
use crate::installations::actions::uninstall;
use crate::installations::context::Context;

pub async fn run(args: &Args, cmd: &UninstallSubcommandArgs) -> anyhow::Result<()> {
    if cmd.packages.is_empty() {
        anyhow::bail!("Specify at least one package to uninstall.");
    }

    let ctx = Context::get_current(args.system)
        .await
        .context("Could not get current installation context.")?;

    let bar = display::spinner("Uninstalling packages...");
    for package_name in &cmd.packages {
        bar.set_message(format!("Uninstalling {}...", package_name.blue()));

        uninstall::uninstall(&ctx, package_name)
            .await
            .context(format!("Could not uninstall package {}.", package_name))?;
    }
    bar.finish_and_clear();

    println!("Done! The following packages were uninstalled:");
    for (index, package_name) in cmd.packages.iter().enumerate() {
        println!(
            "{} {}",
            format!("{}.", index + 1).dimmed(),
            package_name.blue(),
        );
    }

    Ok(())
}
