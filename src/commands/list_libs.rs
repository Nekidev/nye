use anyhow::Context as AnyhowContext;
use colored::Colorize;

use crate::args::{Args, ListSubcommandLibsSubcommandArgs};
use crate::display;
use crate::installations::actions::list;
use crate::installations::context::Context;
use crate::installations::database;

pub async fn run(args: &Args, _cmd: &ListSubcommandLibsSubcommandArgs) -> anyhow::Result<()> {
    let ctx = Context::get_current(args.system)
        .await
        .context("Could not get current installation context.")?;

    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;
    let mut libraries = list::list_libs(&ctx)
        .await
        .context("Could not list all exposed libraries.")?;

    libraries.sort_by_cached_key(|p| p.name.clone());

    if libraries.is_empty() {
        println!("No installed packages exposed any libraries.");
    } else {
        println!(
            "The following libraries are exposed by installed packages (in the current installation context):"
        );

        let mut rows = Vec::with_capacity(libraries.len());

        for (index, library) in libraries.iter().enumerate() {
            let package = library
                .package()
                .exec(&mut db)
                .await
                .context("Could not get exposed library's package.")?;

            rows.push([
                format!("{}.", index + 1).dimmed().to_string(),
                library.name.blue().to_string(),
                format!("@ {} v{}", package.name, package.version),
                library.location.dimmed().to_string(),
            ]);
        }

        let table = display::list_table(rows);
        println!("{table}");
    }

    Ok(())
}
