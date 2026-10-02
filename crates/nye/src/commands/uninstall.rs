use anyhow::Context as AnyhowContext;
use colored::Colorize;

use crate::args::{Args, UninstallSubcommandArgs};
use crate::display;

pub async fn run(args: &Args, cmd: &UninstallSubcommandArgs) -> anyhow::Result<()> {
    if cmd.packages.is_empty() {
        anyhow::bail!("Specify at least one package to uninstall.");
    }

    let nms = args
        .get_namespace()
        .await
        .context("Could not get current namespace.")?;

    let mut installations = Vec::with_capacity(cmd.packages.len());
    let bar = display::spinner("Getting installed packages...");

    for package_name in &cmd.packages {
        let installation = nms
            .get_installation_by_name(package_name)
            .await
            .context("Could not get installation from namespace.")?
            .context(format!("No package called `{package_name}` is installed."))?;

        installations.push(installation);
    }

    for installation in installations {
        let package_name = installation.package_name.clone();
        let package_version = installation.package_version.clone();

        bar.set_message(format!(
            "Uninstalling {}...",
            format!("{package_name} v{package_version}").blue()
        ));

        installation
            .uninstall()
            .await
            .context(format!("Could not uninstall package {package_name}."))?;
    }
    bar.finish_and_clear();

    println!("Done! The following packages were uninstalled:");
    for (index, package_name) in cmd.packages.iter().enumerate() {
        println!("{} {}", format!("{}.", index + 1).dimmed(), package_name.blue(),);
    }

    Ok(())
}
