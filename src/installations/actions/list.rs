use anyhow::Context as AnyhowContext;

use crate::installations::context::Context;
use crate::installations::database::{self, ExposedArtifact, ExposedArtifactKind, Package};

pub async fn list(ctx: &Context) -> anyhow::Result<Vec<Package>> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;

    Package::all()
        .exec(&mut db)
        .await
        .context("Could not list all installed packages.")
}

pub async fn list_artifacts(ctx: &Context, kind: ExposedArtifactKind) -> anyhow::Result<Vec<ExposedArtifact>> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;

    ExposedArtifact::all().filter_by_kind(&kind)
        .exec(&mut db)
        .await
        .context(format!("Could not list all exposed artifacts of kind {kind:?}."))
}
