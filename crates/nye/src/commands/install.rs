use anyhow::Context;
use colored::Colorize;
use nye_namespaces::installations::Installation;
use nye_packages::format::reading::safety::Safety;

use crate::args::{Args, InstallSubcommandArgs};
use crate::display;

pub async fn run(args: &Args, cmd: &InstallSubcommandArgs) -> anyhow::Result<()> {
    let nms = args.get_namespace().await?;

    let bar = display::spinner("Validating packages...");

    let mut installers = vec![];
    for path in &cmd.path {
        bar.set_message(format!("Validating package in {}...", path.display().to_string().blue()));

        let installer = Installation::build_from_path(nms.clone(), path, Safety::default())
            .await
            .context("Could not get installer.")?;
        installer.validate().await.context(format!(
            "The package at {} conflicted with one or more installed packages.",
            path.display()
        ))?;

        installers.push(installer);
    }

    let mut manifests = vec![];
    for installer in installers {
        bar.set_message(format!(
            "Installing {}...",
            format!(
                "{} v{}",
                installer.manifest().package.name,
                installer.manifest().package.version
            )
            .blue()
        ));

        manifests.push(installer.manifest().clone());

        installer
            .install()
            .await
            .context("Could not install package.")?;
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
