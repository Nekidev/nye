//! Toasty CLI for the [`nye-namespaces`] crate.

use anyhow::Context;
use nye_namespaces::database;
use toasty_cli::{Config, ToastyCli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load().context("Could not load configuration.")?;

    let db = database::connect("sqlite:./namespaces.db")
        .await
        .context("Could not connect to local development database.")?;

    let cli = ToastyCli::with_config(db, config);
    cli.parse_and_run()
        .await
        .context("An error occurred while parsing and running the Toasty CLI.")?;

    Ok(())
}
