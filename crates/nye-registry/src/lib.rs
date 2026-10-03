//! Nye registries interface.

use std::path::Path;

use nye_schemas::semver::{Semver, SemverQuery};
use serde::{Deserialize, Serialize};

use crate::meta::RegistryMeta;

pub mod config;
pub mod meta;

/// A package in the registry.
///
/// Given registries don't have actions like "query multiple versions of one package", this type
/// represents a single version of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The package's name.
    pub name: String,
    /// The package's version.
    pub version: Semver,
}

/// An enum wrapper over officially-provided nye registry clients.
pub enum Registry {}

/// A kind of registry.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryKind {}

/// A trait implemented by registry clients.
///
/// It may happen that registry clients become installable as extensions to nye in the future.
/// However, this API is not designed with such plans in mind.
pub trait RegistryBackend {
    /// Get the registry's metadata, holding details such as its name.
    ///
    /// This function is always called when the registry is first used within a session. For
    /// example, at the beginning of the `nye install` command when the registry will be used to
    /// query and/or download packages.
    ///
    /// This is a good place to store registry-specific configurations inside the type on
    /// initialization. For example, an HTTP registry could return a download URL template in a
    /// `/config` endpoint that gets stored for later usage in [`RegistryBackend::download()`].
    ///
    /// Returns:
    /// * `Ok(RegistryMeta)` - The registry's metadata.
    /// * `Err(Error)` - If an error occurred while fetching the registry's configuration.
    fn init(&mut self) -> impl Future<Output = anyhow::Result<RegistryMeta>> + Send + Sync;

    /// Query a package version.
    ///
    /// The package query contains the package's name as the resource. It is on the caller to
    /// ensure the resource is always set. In case the caller does not specify a resource, this
    /// method must fail.
    ///
    /// Arguments:
    /// * `query` - The packge query.
    ///
    /// Returns:
    /// * `Ok(Some(Package))` - If a package was found for the query.
    /// * `Ok(None)` - If no package was found for the query.
    /// * `Err(Error)` - If an error occurred while querying.
    fn query(
        &mut self,
        query: SemverQuery,
    ) -> impl Future<Output = anyhow::Result<Option<Package>>> + Send + Sync;

    /// Downloads a package file to the specified location.
    ///
    /// Arguments:
    /// * `package` - The package to download.
    /// * `output_path` - The path where the package must be downloaded at.
    /// * `on_progress` - A function called when the progress is updated.
    fn download<Phantom>(
        &mut self,
        package: &Package,
        output_path: impl AsRef<Path>,
        on_progress: impl OnProgress<Phantom>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + Sync;
}

/// Trait implemented by [`RegistryBackend::download()`] progress update callbacks.
pub trait OnProgress<Phantom> {
    fn on_progress(&mut self, progress: u64, total: Option<u64>) -> anyhow::Result<()>;
}

/// Phantom type for `FnMut(u64, Option<u64>) -> ()` [`OnProgress`] impls.
#[doc(hidden)]
pub struct PhantomFnMut;
/// Phantom type for `FnMut(u64, Option<u64>) -> Result<(), E>` [`OnProgress`] impls.
#[doc(hidden)]
pub struct PhantomFnMutResult;

impl<F> OnProgress<PhantomFnMut> for F
where
    F: FnMut(u64, Option<u64>),
{
    fn on_progress(&mut self, progress: u64, total: Option<u64>) -> anyhow::Result<()> {
        self(progress, total);
        Ok(())
    }
}

impl<F, E> OnProgress<PhantomFnMutResult> for F
where
    F: FnMut(u64, Option<u64>) -> Result<(), E>,
    E: Into<anyhow::Error>,
{
    fn on_progress(&mut self, progress: u64, total: Option<u64>) -> anyhow::Result<()> {
        self(progress, total).map_err(Into::into)
    }
}
