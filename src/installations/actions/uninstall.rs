use std::path::Path;

use anyhow::Context as AnyhowContext;
use toasty::Transaction;
use tokio::fs;

use crate::installations::context::Context;
use crate::installations::database::{self, ExposedArtifact, ExposedArtifactKind, Package};

pub async fn uninstall(ctx: &Context, name: &str) -> anyhow::Result<()> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;
    let mut db = db
        .transaction()
        .await
        .context("Could not start a database transaction.")?;

    let package = Package::get_by_name(&mut db, name)
        .await
        .context(format!("Could not get package named {name}."))?;

    uninstall_artifacts(ctx, &package, &mut db)
        .await
        .context("Could not uninstall one or more artifacts.")?;

    let package_path = ctx.root.join("pkg").join("store").join(&package.name);
    let version_path = package_path.join(&package.version);

    fs::remove_dir_all(version_path)
        .await
        .context("Could not delete installed package version path.")?;

    let mut remaining_versions = fs::read_dir(&package_path)
        .await
        .context("Could not list remaining installed versions for package.")?;

    if remaining_versions
        .next_entry()
        .await
        .context("Could not get next entry in package's installed versions directory.")?
        .is_none()
    {
        fs::remove_dir(&package_path)
            .await
            .context("Could not delete empty package versions directory.")?;
    }

    package
        .delete()
        .exec(&mut db)
        .await
        .context("Could not remove uninstalled package from state database.")?;

    db.commit()
        .await
        .context("Could not commit state database transaction.")?;

    Ok(())
}

async fn uninstall_artifacts(
    ctx: &Context,
    package: &Package,
    db: &mut Transaction<'_>,
) -> anyhow::Result<()> {
    let artifacts = package
        .exposes()
        .exec(db)
        .await
        .context("Could not get exposed artifacts of package.")?;

    for artifact in artifacts {
        match &artifact.kind {
            ExposedArtifactKind::Binary => uninstall_bin(ctx, &artifact)
                .await
                .context("Could not uninstall exposed binary.")?,
            ExposedArtifactKind::Library => uninstall_lib(ctx, &artifact)
                .await
                .context("Could not uninstall exposed library.")?,
            ExposedArtifactKind::Variable => uninstall_env(ctx, &artifact)
                .await
                .context("Could not uninstall exposed environment variable.")?,
        };
    }

    Ok(())
}

async fn uninstall_bin(ctx: &Context, artifact: &ExposedArtifact) -> anyhow::Result<()> {
    let path = ctx.root.join("bin").join(&artifact.name);

    if fs::try_exists(&path)
        .await
        .context("Could not check if link to exposed binary existed.")?
    {
        fs::remove_file(&path).await.context(format!(
            "Could not delete binary link at `{}`.",
            path.display()
        ))?;
    }

    Ok(())
}

async fn uninstall_lib(ctx: &Context, artifact: &ExposedArtifact) -> anyhow::Result<()> {
    let path = ctx.root.join("lib").join(&artifact.name);

    if fs::try_exists(&path)
        .await
        .context("Could not check if link to exposed library existed.")?
    {
        fs::remove_file(&path).await.context(format!(
            "Could not delete library link at `{}`.",
            path.display()
        ))?;
    }

    Ok(())
}

async fn uninstall_env(ctx: &Context, artifact: &ExposedArtifact) -> anyhow::Result<()> {
    let path = &artifact.location;

    if fs::try_exists(path)
        .await
        .context("Could not check if exposed environment variable values directory existed.")?
    {
        fs::remove_dir_all(path).await.context(format!(
            "Could not delete environment variable values directory at `{path}`."
        ))?;

        let env_var_package_versions_dir_path = ctx
            .root
            .join("env")
            .join(&artifact.name)
            .join(&artifact.package_name);
        if delete_dir_if_empty(env_var_package_versions_dir_path)
            .await
            .context("Could not delete environment variable package versions directory if empty.")?
        {
            let env_var_dir = ctx.root.join("env").join(&artifact.name);
            delete_dir_if_empty(env_var_dir)
                .await
                .context("Could not delete environment variable directory if empty.")?;
        }
    }

    Ok(())
}

async fn delete_dir_if_empty(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    let path = path.as_ref();

    let is_empty = fs::read_dir(path)
        .await
        .context("Could not read entries of directory.")?
        .next_entry()
        .await
        .context("Could not get next entry of directory.")?
        .is_none();

    if is_empty {
        fs::remove_dir(path)
            .await
            .context("Could not delete empty directory.")?;

        Ok(true)
    } else {
        Ok(false)
    }
}
