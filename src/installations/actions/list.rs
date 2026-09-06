use anyhow::Context as AnyhowContext;

use crate::installations::context::Context;
use crate::installations::database::{self, ExposedBin, ExposedLib, Package};

pub async fn list(ctx: &Context) -> anyhow::Result<Vec<Package>> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;

    Package::all()
        .exec(&mut db)
        .await
        .context("Could not list all installed packages.")
}

pub async fn list_bins(ctx: &Context) -> anyhow::Result<Vec<ExposedBin>> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;

    ExposedBin::all()
        .exec(&mut db)
        .await
        .context("Could not list all exposed binaries.")
}

pub async fn list_libs(ctx: &Context) -> anyhow::Result<Vec<ExposedLib>> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;

    ExposedLib::all()
        .exec(&mut db)
        .await
        .context("Could not list all exposed libraries.")
}
