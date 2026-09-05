use anyhow::Context as AnyhowContext;

use crate::installations::context::Context;
use crate::installations::database::{self, Package};

pub async fn list(ctx: &Context) -> anyhow::Result<Vec<Package>> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;

    Package::all()
        .exec(&mut db)
        .await
        .context("Could not list all installed packages.")
}
