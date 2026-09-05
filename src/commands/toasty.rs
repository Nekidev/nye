use anyhow::Context;
use toasty_cli::{Config, ToastyCli};

use crate::args::{Args, ToastySubcommandArgs};
use crate::installations::database;

pub async fn run(_args: &Args, cmd: &ToastySubcommandArgs) -> anyhow::Result<()> {
    let config = Config::load()?;

    let mut args = vec![String::from("toasty"), String::from("migration")];
    args.append(&mut cmd.args.clone());

    let db = database::connect("sqlite:./state.db")
        .await
        .context("Could not connect to development database at ./state.db.")?;

    let cli = ToastyCli::with_config(db, config);
    cli.parse_from(args).await?;

    Ok(())
}
