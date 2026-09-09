use std::path::Path;

use anyhow::Context;
use toasty_cli::{Config, ToastyCli};

use crate::args::{Args, ToastySubcommandArgs};
use crate::installations::database;

pub async fn run(_args: &Args, cmd: &ToastySubcommandArgs) -> anyhow::Result<()> {
    let config = Config::load_from(Path::new("src/installations/Toasty.toml"))?;

    let mut args = vec![String::from("toasty"), String::from("migration")];
    args.append(&mut cmd.args.clone());

    let db = database::connect(cmd.database_url.to_string())
        .await
        .context(format!(
            "Could not connect to development database at `{}`.",
            cmd.database_url
        ))?;

    let cli = ToastyCli::with_config(db, config);
    cli.parse_from(args).await?;

    Ok(())
}
