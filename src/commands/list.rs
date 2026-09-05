use anyhow::Context as AnyhowContext;
use colored::Colorize;

use crate::args::{Args, ListSubcommandArgs};
use crate::installations::actions::list;
use crate::installations::context::Context;

pub async fn run(args: &Args, _cmd: &ListSubcommandArgs) -> anyhow::Result<()> {
    let ctx = Context::get_current(args.system)
        .await
        .context("Could not get current installation context.")?;

    let mut packages = list::list(&ctx)
        .await
        .context("Could not list all installed packages.")?;

    packages.sort_by_cached_key(|p| p.name.clone());

    if packages.is_empty() {
        println!("You currently have no packages installed.");
    } else {
        println!(
            "The following packages are currenly installed (in the current installation context):"
        );
        for (index, package) in packages.iter().enumerate() {
            println!(
                "{} {}  {}",
                format!("{}.", index + 1).dimmed(),
                format!("{} v{}", package.name, package.version).blue(),
                ctx.get_package_installation_path(&package.name, &package.version)
                    .context("Could not get package's installation path.")?
                    .display()
                    .to_string()
                    .dimmed(),
            );
        }
    }


    Ok(())
}
