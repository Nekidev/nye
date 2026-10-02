//! Install, uninstall, and manage installed packages.
//!
//! # Paths
//!
//! Packages can write to the following directories within the namespace's root:
//!
//! ```text
//! Symlinks:
//!     /bin/{link}
//!     /lib/{link}
//!     /env/{variable-name}/{package-name}/{package-version}
//!
//! Package data:
//!     /pkg/store/{package-name}/{package-version}
//! ```

// compile_error!(concat!(
//     "TODO: Add cleanup module that revises the system's state and cleans up any unregistered ",
//     "packages. It'll also require to be able to restore a state database from the system itself,
// ",     "i.e. a two-way recovery."
// ));

use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use nye_packages::format::reading::safety::Safety;
use nye_packages::format::reading::{NyeFileSeekableReader, ReadableSeekable};
use nye_packages::manifest::Manifest;
use nye_schemas::semver::Semver;
use tokio::fs::File;

use crate::Namespace;
use crate::database::Version;
use crate::installations::installing::{InstallationBuilder, InstallationCrafter};

pub mod installing;
pub mod uninstalling;
pub mod wrappers;

/// An existing package installation.
#[derive(Debug, Clone)]
pub struct Installation {
    namespace: Namespace,

    /// The root path to the installation.
    pub path: PathBuf,

    pub package_name: String,
    pub package_version: Semver,
}

impl Installation {
    /// Create an installation from a package file.
    ///
    /// Arguments:
    /// * `namespace` - The namespace to install the package in.
    /// * `package` - The package file reader to install from.
    pub fn build<R>(
        namespace: Namespace,
        package: NyeFileSeekableReader<R>,
    ) -> InstallationBuilder<R>
    where
        R: ReadableSeekable,
    {
        InstallationBuilder::new(namespace, package)
    }

    /// Create an installation from a package file's path.
    ///
    /// Arguments:
    /// * `namespace` - The namespace to install the package in.
    /// * `path` - The path to the package file to install from.
    /// * `safety` - The safety rules to open the package file with.
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

    /// Create an installation by specifying the package's contents by hand.
    ///
    /// Installation crafters assume you have taken adequate steps to ensure the package you are
    /// installing is safe.
    ///
    /// Arguments:
    /// * `namespace` - The namespace to install the package in.
    /// * `manifest` - The package's manifest.
    pub fn craft(namespace: Namespace, manifest: Manifest) -> InstallationCrafter {
        InstallationCrafter::new(namespace, manifest)
    }

    pub async fn uninstall(self) -> anyhow::Result<()> {
        uninstalling::uninstall(self).await
    }

    pub(crate) fn from_version(
        namespace: Namespace,
        version: Version,
    ) -> anyhow::Result<Installation> {
        Ok(Installation {
            namespace,
            path: PathBuf::from(&version.path),
            package_name: version.package_name.clone(),
            package_version: Semver::from_str(&version.number)
                .context("An installed package had an invalid semver version.")?,
        })
    }
}

/// Installation-related associated functions and methods.
impl Namespace {
    /// Create an installation from a package file.
    ///
    /// Arguments:
    /// * `package` - The package file reader to install from.
    pub fn build_installation<R>(&self, package: NyeFileSeekableReader<R>) -> InstallationBuilder<R>
    where
        R: ReadableSeekable,
    {
        Installation::build(self.clone(), package)
    }

    /// Create an installation from a package file's path.
    ///
    /// Arguments:
    /// * `path` - The path to the package file to install from.
    /// * `safety` - The safety rules to open the package file with.
    pub async fn build_installation_from_path(
        &self,
        path: impl AsRef<Path>,
        safety: Safety,
    ) -> anyhow::Result<InstallationBuilder<File>> {
        Installation::build_from_path(self.clone(), path, safety).await
    }

    /// Create an installation by specifying the package's contents by hand.
    ///
    /// Installation crafters assume you have taken adequate steps to ensure the package you are
    /// installing is safe.
    ///
    /// Arguments:
    /// * `manifest` - The package's manifest.
    pub fn craft_installation(&self, manifest: Manifest) -> InstallationCrafter {
        InstallationCrafter::new(self.clone(), manifest)
    }

    /// Get all installations from the namespace's state database.
    pub async fn get_installations(&self) -> anyhow::Result<Vec<Installation>> {
        let mut connection = self
            .state()
            .connection()
            .await
            .context("Could not get a connection to the namespace's state database.")?;

        let versions = Version::all()
            .exec(&mut connection)
            .await
            .context("Could not list all installed package versions.")?;

        let mut result = Vec::with_capacity(versions.len());
        for version in versions {
            result.push(
                Installation::from_version(self.clone(), version)
                    .context("An installation was invalid.")?,
            );
        }

        Ok(result)
    }

    /// Get an installation from the namespace's state database by package name.
    ///
    /// Arguments:
    /// * `name` - The installed package's name.
    ///
    /// Returns:
    /// * `Ok(Some(Installation))` - If the installation was found,
    /// * `Ok(None)` - If no installation was found for the specified name,
    /// * `Err(Error)` - If an error occurred while querying the database.
    pub async fn get_installation_by_name(
        &self,
        name: impl Into<String>,
    ) -> anyhow::Result<Option<Installation>> {
        let mut connection = self
            .state()
            .connection()
            .await
            .context("Could not get a connection to the namespace's state database.")?;

        let version = Version::filter_by_package_name(name.into())
            .first()
            .exec(&mut connection)
            .await
            .context("Could not query package installation in namespace's state database.")?;

        if let Some(version) = version {
            Ok(Some(Installation::from_version(self.clone(), version)?))
        } else {
            Ok(None)
        }
    }

    /// Get an installation from the namespace's state database by package name and version.
    ///
    /// Arguments:
    /// * `name` - The installed package's name.
    /// * `version` - The installed package's version number.
    ///
    /// Returns:
    /// * `Ok(Some(Installation))` - If the installation was found,
    /// * `Ok(None)` - If no installation was found for the specified name and version,
    /// * `Err(Error)` - If an error occurred while querying the database.
    pub async fn get_installation_by_name_and_version(
        &self,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> anyhow::Result<Option<Installation>> {
        let mut connection = self
            .state()
            .connection()
            .await
            .context("Could not get a connection to the namespace's state database.")?;

        let version = Version::filter_by_package_name_and_number(name.into(), version.into())
            .first()
            .exec(&mut connection)
            .await
            .context("Could not query package installation in namespace's state database.")?;

        if let Some(version) = version {
            Ok(Some(Installation::from_version(self.clone(), version)?))
        } else {
            Ok(None)
        }
    }
}
