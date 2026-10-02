use std::path::Path;

use anyhow::Context;
use tokio::fs;

use crate::Namespace;
use crate::database::{Artifact, ArtifactKind, Package, Version};
use crate::installations::Installation;

pub async fn uninstall(installation: Installation) -> anyhow::Result<()> {
    let mut connection = installation
        .namespace
        .state()
        .connection()
        .await
        .context("Could not get namespace state database's connection.")?;
    let mut transaction = connection
        .transaction()
        .await
        .context("Could not get transaction for namespace's state database.")?;

    fs::remove_dir_all(&installation.path)
        .await
        .context("Could not remove installation's directory.")?;
    remove_dir_if_empty(
        installation
            .path
            .parent()
            .context("Installation had no parent.")?,
    )
    .await
    .context("Could not delete installation's parent directory if empty.")?;

    let artifacts = Artifact::filter_by_package_name_and_version_number(
        &installation.package_name,
        installation.package_version.to_string(),
    )
    .exec(&mut transaction)
    .await
    .context("Could not get exposed artifacts.")?;
    for artifact in artifacts {
        unexpose_artifact(&installation.namespace, artifact)
            .await
            .context("Could not unexpose artifact.")?;
    }

    let package = Package::get_by_name(&mut transaction, &installation.package_name)
        .await
        .context("Could not get package from namespace's state database.")?;
    let version = Version::get_by_package_name_and_number(
        &mut transaction,
        &installation.package_name,
        installation.package_version.to_string(),
    )
    .await
    .context("Could not get package version from namespace's state database.")?;

    version
        .delete()
        .exec(&mut transaction)
        .await
        .context("Could not delete package version from namespace's state database.")?;

    if package
        .versions()
        .count()
        .exec(&mut transaction)
        .await
        .context("Could not count package's installed versions.")?
        == 0
    {
        package
            .delete()
            .exec(&mut transaction)
            .await
            .context("Could not delete package from namespace's state database.")?;
    }

    transaction
        .commit()
        .await
        .context("Could not commit transaction to namespace's state database.")?;

    Ok(())
}

async fn remove_dir_if_empty(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    let mut readdir = fs::read_dir(&path)
        .await
        .context("Could not read directory.")?;
    let entry = readdir
        .next_entry()
        .await
        .context("Could not read next entry.")?;

    if entry.is_none() {
        fs::remove_dir(&path)
            .await
            .context("Could not delete empty directory.")?;
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn unexpose_artifact(namespace: &Namespace, artifact: Artifact) -> anyhow::Result<()> {
    match &artifact.kind {
        ArtifactKind::Bin => unexpose_artifact_bin(namespace, artifact).await,
        ArtifactKind::Lib => unexpose_artifact_lib(namespace, artifact).await,
        ArtifactKind::Env => unexpose_artifact_env(namespace, artifact).await,
        ArtifactKind::Var => Ok(()),
    }
}

async fn unexpose_artifact_bin(namespace: &Namespace, artifact: Artifact) -> anyhow::Result<()> {
    let path = namespace.path().join("bin").join(&artifact.link);
    fs::remove_file(path)
        .await
        .context("Could not unexpose binary artifact.")?;

    Ok(())
}

async fn unexpose_artifact_lib(namespace: &Namespace, artifact: Artifact) -> anyhow::Result<()> {
    let path = namespace.path().join("lib").join(&artifact.link);
    fs::remove_file(path)
        .await
        .context("Could not unexpose library artifact.")?;

    Ok(())
}

async fn unexpose_artifact_env(namespace: &Namespace, artifact: Artifact) -> anyhow::Result<()> {
    let path_root = namespace.path().join("env").join(artifact.link);
    let path_package = path_root.join(artifact.package_name);
    let path_version = path_package.join(artifact.version_number);

    fs::remove_file(&path_version)
        .await
        .context("Could not remove env var's symlink to values.")?;
    remove_dir_if_empty(path_package)
        .await
        .context("Could not delete env var's symlink to values parent dir if empty.")?;
    remove_dir_if_empty(path_root)
        .await
        .context("Could not delete env var's symlink to values parent's parent dir if empty.")?;

    Ok(())
}
