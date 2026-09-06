use anyhow::Context as AnyhowContext;
use colored::Colorize;

use crate::args::{Args, ListSubcommandBinsSubcommandArgs};
use crate::display;
use crate::installations::actions::list;
use crate::installations::context::Context;
use crate::installations::database;

pub async fn run(args: &Args, _cmd: &ListSubcommandBinsSubcommandArgs) -> anyhow::Result<()> {
    let ctx = Context::get_current(args.system)
        .await
        .context("Could not get current installation context.")?;

    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;
    let mut binaries = list::list_bins(&ctx)
        .await
        .context("Could not list all exposed binaries.")?;

    binaries.sort_by_cached_key(|p| p.name.clone());

    if binaries.is_empty() {
        println!("No installed packages exposed any binaries.");
    } else {
        println!(
            "The following binaries are exposed by installed packages (in the current installation context):"
        );

        let mut rows = Vec::with_capacity(binaries.len());

        for (index, binary) in binaries.iter().enumerate() {
            let package = binary
                .package()
                .exec(&mut db)
                .await
                .context("Could not get exposed binary's package.")?;

            rows.push([
                format!("{}.", index + 1).dimmed().to_string(),
                binary.name.blue().to_string(),
                format!("@ {} v{}", package.name, package.version),
                binary.location.dimmed().to_string(),
            ]);
        }

        let table = display::list_table(rows);
        println!("{table}");
    }

    Ok(())
}
