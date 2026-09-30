//! Install, uninstall, and manage installed packages.
//!

// compile_error!(concat!(
//     "TODO: Add cleanup module that revises the system's state and cleans up any unregistered ",
//     "packages. It'll also require to be able to restore a state database from the system itself, ",
//     "i.e. a two-way recovery."
// ));

use std::path::{Path, PathBuf};

use anyhow::Context;
use nye_packages::format::reading::safety::Safety;
use nye_packages::format::reading::{NyeFileSeekableReader, ReadableSeekable};
use nye_packages::manifest::Manifest;
use nye_schemas::semver::Semver;
use tokio::fs::File;

use crate::Namespace;
use crate::installations::installing::{InstallationBuilder, InstallationCrafter};

pub mod installing;
pub mod wrappers;

/// An existing package installation.
pub struct Installation {
    pub path: PathBuf,

    pub package_name: String,
    pub package_version: Semver,
}

impl Installation {
    pub fn build<R>(
        namespace: Namespace,
        package: NyeFileSeekableReader<R>,
    ) -> InstallationBuilder<R>
    where
        R: ReadableSeekable,
    {
        InstallationBuilder::new(namespace, package)
    }

    pub async fn build_from_path(
        namespace: Namespace,
        path: impl AsRef<Path>,
        safety: Safety,
    ) -> anyhow::Result<InstallationBuilder<File>> {
        let file = File::open(path)
            .await
            .context("Could not open the package file at the specified path.")?;
        let reader = NyeFileSeekableReader::open(file, safety)
            .await
            .context("Could not open the package file from the file at the specified path.")?;

        Ok(InstallationBuilder::new(namespace, reader))
    }

    pub fn craft(namespace: Namespace, manifest: Manifest) -> InstallationCrafter {
        InstallationCrafter::new(namespace, manifest)
    }
}
