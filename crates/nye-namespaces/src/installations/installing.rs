//! Install a package into a namespace.
//!
//! Packages are installed using really lax permissions on the package files themselves, relying on
//! parent directories to hold stricter permissions preventing misuse. It's on the caller to ensure
//! those stricter permissions are properly set.
//!
//! # Usage
//!
//! This module provides two ways to install a package into a namespace:
//!
//! 1. [`InstallationBuilder`]: Install a package and check for collisions reading from a `.nye`
//!    package file.
//! 2. [`InstallationCrafter`]: Manually specify the package's manifest and files from the system to
//!    create and install a package without a package file.
//!
//! ## [`InstallationBuilder`]
//!
//! This type is the one you'll likely use most of the time. It's created with
//! [`Installation::build()`] or [`Installation::build_from_path()`] and installs a `.nye` package
//! file.
//!
//! ```
//! let file = File::open("package.nye").await?;
//! let reader = NyeFileSeekableReader::open(file, Safety::default()).await?;
//! let namespace = Namespace::get_for_current_user().await?;
//!
//! let installer = Installation::build(namespace, reader);
//! ```
//!
//! However, that is a common pattern so you can use [`Installation::build_from_path()`] instead.
//!
//! ```
//! let namespace = Namespace::get_for_current_user().await?;
//!
//! let installer = Installation::build_from_path(namespace, "package.nye", Safety::default()).await?;
//! ```
//!
//! Once you have your installation builder created, call [`InstallationBuilder::validate()`] to
//! ensure there are no conflicts with other installed packages.
//!
//! ```
//! installer.validate().await?;
//! ```
//!
//! Once that call has passed, you can proceed to running [`InstallationBuilder::install()`].
//!
//! ```
//! let installation = installer.install().await?;
//! ```
//!
//! ### Optimizing via Shrinking
//!
//! When a package file is being extracted by calling [`InstallationBuilder::install()`], almost all
//! of the disk usage of the package file gets duplicated. This is many times not desireable since
//! big package files can end up exhausting the disk space. To counter this, you can use
//! [`InstallationBuilder::install_while_shrinking()`].
//!
//! The method extracts files starting from the end of the package file and cuts them off the
//! package file as they're extracted into their installation location. This way, you can reduce the
//! amount of disk storage used during the extraction process.
//!
//! The method takes a `shrinker` argument, which takes an asynchronous function that you must make
//! implement the file shrinking. For example, with a tokio [`File`]:
//!
//! ```
//! let file = File::open("package.nye").await?;
//! let file_copy = file.try_clone().await?;
//! let mut file_size = file.metadata().await?.len();
//! let reader = NyeFileSeekableReader::open(file_copy, Safety::default()).await?;
//!
//! let namespace = Namespace::get_for_current_user().await?;
//! let installer = Installation::build(namespace, reader);
//!
//! installer.validate().await?;
//!
//! let installation = installer.install_while_shrinking(async |amount| {
//!     file_size -= amount;
//!     file.set_len(file_size).await
//! }).await?;
//! ```
//!
//! [`NyeFileSeekableReader`] comes from the [`nye-packages`] crate.
//!
//! ## [`InstallationCrafter`]
//!
//! This type allows you to specify files from the local filesystem and a package manifest to
//! install a package without a package file. Unlike [`InstallationBuilder`], which is used for
//! untrusted files, this type is built for local environments (read "nye projects") containing
//! only trusted files. Therefore, it does not take a [`Safety`] parameter nor runs any safety
//! checks other than basic collision validation.
//!
//! The type is created with [`Installation::craft()`] as follows:
//!
//! ```
//! let manifest = Manifest { ... };
//! let namespace = Namespace::get_for_current_user().await?;
//!
//! let mut installer = Installation::craft(namespace, manifest);
//! ```
//!
//! To add files to the crafter, use either [`InstallationCrafter::add_file()`] (returns `&mut
//! self`) or [`InstallationCrafter::with_file()`] (returns `self`).
//!
//! ```
//! let file = InstallationCrafterFile::new(
//!     "my-file-name",
//!     NyeFileEntryKind::Bin,
//!     File::open("my-file").await?
//! );
//! installer.add_file(file);
//! ```
//!
//! The [`NyeFileEntryKind`] type comes from the [`nye-packages`] crate.
//!
//! Once you have fully built your installation crafter, validate that there are no collisions by
//! calling [`InstallationCrafter::validate()`].
//!
//! ```
//! installer.validate().await?;
//! ```
//!
//! Once that has run successfully, you can proceed to install the package with
//! [`InstallationCrafter::install()`]. This type does not have an `install_while_shrinking()`
//! method due to the lack of file to shrink.
//!
//! [`nye-packages`]: nye_packages
//! [`Safety`]: nye_packages::format::reading::safety::Safety

use std::collections::HashMap;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::Context;
use askama::Template;
use nye_packages::format::NyeFileEntryKind;
use nye_packages::format::reading::{NyeFileSeekableReader, ReadableSeekable};
use nye_packages::manifest::{Manifest, ManifestConsumesEnv, ManifestExposesArtifact};
use nye_utils::time;
use toasty::Executor;
use tokio::fs::{self, File};
use tokio::io::{self, AsyncRead};

use crate::Namespace;
use crate::database::{Artifact, ArtifactKind, Package, Version};
use crate::installations::Installation;
use crate::installations::wrappers::Wrapper;

/// A wrapper over a function type that shrinks a package file.
///
/// This trait is implemented on the following function types:
/// * `AsyncFnMut(u64) -> ()`
/// * `AsyncFnMut(u64) -> Result<(), impl Into<anyhow::Error>>`
pub trait Shrinker<R> {
    /// Shrinks the package file backwards.
    ///
    /// Arguments:
    /// * `amount` - The amount of bytes to cut off the package file's end.
    fn shrink(&mut self, amount: u64) -> impl std::future::Future<Output = anyhow::Result<()>>;
}

impl<F> Shrinker<()> for F
where
    F: AsyncFnMut(u64),
{
    async fn shrink(&mut self, amount: u64) -> anyhow::Result<()> {
        self(amount).await;
        Ok(())
    }
}

impl<F, E> Shrinker<anyhow::Result<()>> for F
where
    F: AsyncFnMut(u64) -> Result<(), E>,
    E: Into<anyhow::Error>,
{
    async fn shrink(&mut self, amount: u64) -> anyhow::Result<()> {
        self(amount).await.map_err(Into::into)
    }
}

/// Install packages from a package file.
///
/// This struct lets you install packages from package files and ensure no conflicts exist before
/// doing so.
///
/// You can either instantiate this struct from [`InstallationBuilder::new()`] or directly from
/// [`Installation::build()`] or [`Installation::build_from_path()`]. To make life happier,
/// [`Installation`] will provide more ergonomic methods to create builders than the builder type
/// itself.
///
/// You can also "craft" installations by hand to install them directly instead of installing from
/// a package file using [`InstallationCrafter`].
pub struct InstallationBuilder<R>
where
    R: ReadableSeekable,
{
    package: NyeFileSeekableReader<R>,
    namespace: Namespace,
}

impl<R> InstallationBuilder<R>
where
    R: ReadableSeekable,
{
    /// Returns the package's manifest.
    pub fn manifest(&self) -> &Manifest {
        self.package.manifest()
    }
}

impl<R> InstallationBuilder<R>
where
    R: ReadableSeekable,
{
    /// Create a new installation builder.
    ///
    /// You're more likely to want to create instances of this type by calling
    /// [`Installation::build()`] instead.
    ///
    /// Arguments:
    /// * `namespace` - The namespace the package will be installed in.
    /// * `package` - The already-opened nye package file seekable reader.
    pub fn new(namespace: Namespace, package: NyeFileSeekableReader<R>) -> Self {
        Self { package, namespace }
    }

    /// Ensure there are no conflicts with the namespace's current state.
    ///
    /// If there are any, this function will return an error.
    pub async fn validate(&self) -> anyhow::Result<()> {
        check_collisions(&self.namespace, self.package.manifest()).await
    }

    /// Install the package file in the namespace.
    ///
    /// This method is not atomic. Failures may mean partial state, which at the moment requires
    /// manual cleaning.
    ///
    /// Also see [`InstallationBuilder::install_while_shrinking()`], which is more disk
    /// space-efficient than this method at the cost of destroying the original package file.
    ///
    /// Returns:
    /// * `Ok(Installation)` - If the installation succeeded completely.
    /// * `Err(anyhow::Error)` - If the installation failed, partially or completely.
    pub async fn install(self) -> anyhow::Result<Installation> {
        Self::install_while_shrinking(self, async |_| anyhow::Ok(())).await
    }

    /// Install the package file in the namespace while removing extracted files from the original
    /// package file.
    ///
    /// This method is not atomic. Failures may mean partial state, which at the moment requires
    /// manual cleaning.
    ///
    /// For example,
    /// ```
    /// let file = File::open("package.nye").await?;
    /// let file_copy = file.try_clone().await?;
    /// let mut file_size = file.metadata().await?.len();
    /// let reader = NyeFileSeekableReader::open(file_copy, Safety::default()).await?;
    ///
    /// let namespace = Namespace::get_for_current_user().await?;
    /// let installer = Installation::build(namespace, reader);
    ///
    /// let installation = installer.install_while_shrinking(async |amount| {
    ///     file_size -= amount;
    ///     file.set_len(file_size).await
    /// }).await?;
    /// ```
    ///
    /// Also see [`InstallationBuilder::install()`] for an alias to this method with a no-op
    /// shrinker.
    ///
    /// Arguments:
    /// * `shrink` - The file-shrinking function.
    ///
    /// Returns:
    /// * `Ok(Installation)` - If the installation succeeded completely.
    /// * `Err(anyhow::Error)` - If the installation failed, partially or completely.
    pub async fn install_while_shrinking<F, A>(mut self, shrink: F) -> anyhow::Result<Installation>
    where
        F: Shrinker<A>,
    {
        let manifest = self.package.manifest();
        let installation = self
            .namespace
            .path()
            .join("pkg")
            .join("store")
            .join(&manifest.package.name)
            .join(manifest.package.version.to_string());

        self.extract_files(&installation, shrink)
            .await
            .context("Could not extract package file's files.")?;

        let manifest = self.package.manifest();
        expose_installation(&self.namespace, manifest, &installation)
            .await
            .context("Could not expose installation.")?;

        Ok(Installation {
            path: installation,
            package_name: manifest.package.name.clone(),
            package_version: manifest.package.version.clone(),
        })
    }

    /// Extracts the package's files into the package's installation folder.
    ///
    /// Arguments:
    /// * `root` - The absolute path to the installation's root, e.g.
    ///   `/usr/root/pkg/store/busybox/0.0.0`.
    /// * `shrink` - The shrinker passed to [`InstallationBuilder::install_while_shrinking()`].
    async fn extract_files<F, A>(
        &mut self,
        root: impl AsRef<Path>,
        mut shrink: F,
    ) -> anyhow::Result<()>
    where
        F: Shrinker<A>,
    {
        let mut i = self.package.entries().len();
        while i > 0
            && let Some(mut entry) = self
                .package
                .get_entry_by_index(i - 1)
                .await
                .context("Could not read next entry.")?
        {
            let output_path = root
                .as_ref()
                .join(entry.kind.to_string())
                .join(entry.name.to_string());
            let output_path_parent = output_path
                .parent()
                .context("The output (extraction) path of a file had no parent?")?;
            fs::create_dir_all(output_path_parent)
                .await
                .context("Could not ensure directories existed for file extraction.")?;

            let mut output = File::create_new(output_path)
                .await
                .context("Could not create output file.")?;
            io::copy(&mut entry, &mut output)
                .await
                .context("Could not extract file to output.")?;

            output
                .set_permissions(Permissions::from_mode(0o755))
                .await
                .context("Could not update permissions of extracted file.")?;

            i -= 1;
            shrink
                .shrink(entry.size)
                .await
                .context("The file shrinking function returned an error.")?;
        }

        Ok(())
    }
}

/// Install packages by specifying their contents by hand.
///
/// Given this struct is intended to be used with controlled packages, this type does not provide
/// any [`Safety`] checks. It's on the caller to ensure the package won't harm the system.
pub struct InstallationCrafter {
    namespace: Namespace,
    manifest: Manifest,
    files: Vec<InstallationCrafterFile>,
}

impl InstallationCrafter {
    /// Creates a new installation crafter.
    ///
    /// You're more likely to want to initiate an instance of this type by calling [`Installation`]
    /// since its API is more focused on end-user ergonomics.
    ///
    /// Also see [`InstallationCrafter::with_capacity()`].
    ///
    /// Arguments:
    /// * `namespace` - The namespace into which to install the package once crafted.
    /// * `manifest` - The package's manifest.
    pub fn new(namespace: Namespace, manifest: impl Into<Manifest>) -> Self {
        Self {
            namespace,
            manifest: manifest.into(),
            files: Vec::new(),
        }
    }

    /// Creates a new installation crafter preallocating file metadata memory.
    ///
    /// Arguments:
    /// * `namespace` - The namespace into which to install the package once crafted.
    /// * `manifest` - The package's manifest.
    /// * `capacity` - The minimum amount of file metadata storage to preallocate.
    pub fn with_capacity(
        namespace: Namespace,
        manifest: impl Into<Manifest>,
        capacity: usize,
    ) -> Self {
        Self {
            namespace,
            manifest: manifest.into(),
            files: Vec::with_capacity(capacity),
        }
    }

    /// Add a file to the installation taking and returning a mutable reference to itself.
    ///
    /// It takes and returns a mutable reference to itself, useful for dynamic building. For a
    /// static chain (`builder = builder.add_file().add_file()`), take a look at
    /// [`InstallationCrafter::with_file()`].
    ///
    /// Arguments:
    /// * `file` - The file to add to the installation.
    pub fn add_file(&mut self, file: impl Into<InstallationCrafterFile>) -> &mut Self {
        self.files.push(file.into());
        self
    }

    /// Add a file to the installation taking and returning its own ownership.
    ///
    /// This method is useful for static builder-style chaining (`builder =
    /// builder.with_file().with_file()`). For dynamic building, take a look at
    /// [`InstallationCrafter::add_file()`].
    ///
    /// Arguments:
    /// * `file` - The file to add to the installation.
    pub fn with_file(mut self, file: impl Into<InstallationCrafterFile>) -> Self {
        self.files.push(file.into());
        self
    }

    /// Ensure there are no conflicts with the namespace's current state.
    ///
    /// If there are any, this function will return an error.
    pub async fn validate(&self) -> anyhow::Result<()> {
        check_collisions(&self.namespace, &self.manifest).await
    }

    /// Finish and install the crafted package into the system.
    ///
    /// This method does not validate collisions before installing. Make sure to call
    /// [`InstallationCrafter::validate()`] by hand.
    ///
    /// This method is not atomic. Failures may mean partial state, which at the moment requires
    /// manual cleaning.
    ///
    /// Returns:
    /// * `Ok(Installation)` - If the installation succeeded completely.
    /// * `Err(anyhow::Error)` - If the installation failed, partially or completely.
    pub async fn install(mut self) -> anyhow::Result<Installation> {
        let installation = self
            .namespace
            .path()
            .join("pkg")
            .join("store")
            .join(&self.manifest.package.name)
            .join(self.manifest.package.version.to_string());

        self.extract_files(&installation)
            .await
            .context("Could not extract files.")?;

        expose_installation(&self.namespace, &self.manifest, &installation)
            .await
            .context("Could not expose installation.")?;

        Ok(Installation {
            path: installation,
            package_name: self.manifest.package.name,
            package_version: self.manifest.package.version,
        })
    }

    /// Extracts the package's files into the package's installation folder.
    ///
    /// Arguments:
    /// * `root` - The absolute path to the installation's root, e.g.
    ///   `/usr/root/pkg/store/busybox/0.0.0`.
    async fn extract_files(&mut self, installation: impl AsRef<Path>) -> anyhow::Result<()> {
        for file in &mut self.files {
            let output_path = installation
                .as_ref()
                .join(file.kind.to_string())
                .join(&file.name);
            let output_path_parent = output_path
                .parent()
                .context("The output (extraction) path of a file had no parent?")?;
            fs::create_dir_all(output_path_parent)
                .await
                .context("Could not ensure directories existed for file extraction.")?;

            let mut output = File::create_new(output_path)
                .await
                .context("Could not create output file.")?;
            io::copy(&mut file.file, &mut output)
                .await
                .context("Could not extract file to output.")?;

            output
                .set_permissions(Permissions::from_mode(0o755))
                .await
                .context("Could not update permissions of extracted file.")?;
        }

        Ok(())
    }
}

/// A file to install with [`InstallationCrafter`].
pub struct InstallationCrafterFile {
    name: String,
    kind: NyeFileEntryKind,
    file: Box<dyn AsyncRead + Unpin>,
}

impl InstallationCrafterFile {
    /// Create a new installation crafter file.
    ///
    /// Arguments:
    /// * `name` - The name of the file, without file kind prefixes (i.e. no `bin/` nor `lib/` root
    ///   directories).
    /// * `kind` - The file's kind.
    /// * `file` - The file to read the contents from.
    pub fn new(
        name: impl Into<String>,
        kind: impl Into<NyeFileEntryKind>,
        file: impl AsyncRead + Unpin + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            file: Box::new(file),
        }
    }
}

/// Ensures no collisions happen between a provided package manifest and a namespace's state.
///
/// This function runs a few checks:
/// 1. Makes sure no same version for the package is already installed.
/// 2. Makes sure no other artifacts collide in links.
///
/// Arguments:
/// * `namespace` - The namespace to check for collisions in.
/// * `manifest` - The package to install's manifest.
///
/// Returns:
/// * `Ok(())` - If no collisions are found.
/// * `Err(anyhow::Error)` - If at least one collision is found.
async fn check_collisions(namespace: &Namespace, manifest: &Manifest) -> anyhow::Result<()> {
    let mut connection = namespace
        .state
        .connection()
        .await
        .context("Could not get connection to state database.")?;

    let package_name = &manifest.package.name;

    let version = Version::filter_by_package_name(package_name)
        .first()
        .exec(&mut connection)
        .await
        .context("Could not get conflicting versions from state database.")?;

    if version.is_some() {
        anyhow::bail!("There is already a package installed with this name.");
    }

    check_artifact_collisions(&manifest.exposes.bin, ArtifactKind::Bin, &mut connection)
        .await
        .context("One or more binary collisions were found.")?;
    check_artifact_collisions(&manifest.exposes.lib, ArtifactKind::Lib, &mut connection)
        .await
        .context("One or more library collisions were found.")?;

    Ok(())
}

/// Ensures no artifacts exposed by a manifest collide with the current namespace's state.
///
/// Arguments:
/// * `artifacts` - The artifacts exposed by the package manifest.
/// * `artifact_kind` - The kind of the artifacts.
/// * `connection` - The connection to the namespace's state database.
///
/// Returns:
/// * `Ok(())` - If no collisions are found.
/// * `Err(anyhow::Error)` - If at least one collision is found.
async fn check_artifact_collisions(
    artifacts: &[ManifestExposesArtifact],
    artifact_kind: impl Into<ArtifactKind>,
    connection: &mut dyn Executor,
) -> anyhow::Result<()> {
    let kind = artifact_kind.into();

    let mut links = Vec::new();
    for artifact in artifacts {
        links.push(artifact.link.clone());
    }

    let query_1 = Artifact::fields().link().in_list(links);
    let query_2 = Artifact::fields().kind().eq(kind);
    let conflicts = Artifact::filter(query_1.and(query_2))
        .exec(connection)
        .await
        .context(format!("Could not query conflicting {kind}s."))?;
    if let Some(artifact) = conflicts.first() {
        anyhow::bail!(
            "The linked {} artifact {} conflicts with an artifact of the same name and kind installed by package {}.",
            artifact.kind,
            artifact.link,
            artifact.package_name
        );
    }

    Ok(())
}

/// Exposes an installed package's artifacts and stores it in the namespace's state database.
///
/// Arguments:
/// * `namespace` - The namespace the package was installed in.
/// * `manifest` - The package's manifest.
/// * `installation` - The absolute path to the directory where the package was installed.
///
/// Returns:
/// * `Ok(())` - If exposing the package's installation was fully successful.
/// * `Err(anyhow::Error)` - If exposing the package's installation was partially or fully
///   unsuccessful.
async fn expose_installation(
    namespace: &Namespace,
    manifest: &Manifest,
    installation: impl AsRef<Path>,
) -> anyhow::Result<()> {
    expose_bins(namespace, manifest, &installation)
        .await
        .context("Could not expose binaries.")?;
    expose_libs(namespace, manifest, &installation)
        .await
        .context("Could not expose libraries.")?;
    expose_envs(namespace, manifest, &installation)
        .await
        .context("Could not expose environment variables.")?;

    update_state(namespace, manifest, &installation)
        .await
        .context("Could not update namespace's state database.")?;

    Ok(())
}

/// Exposes an installed package's binaries.
///
/// This function creates binary wrappers ([`Wrapper`]) and stores them as executables in the
/// namespace's bin directory. It does not check for collisions, so it's on the caller to ensure
/// none occur.
/// 
/// Wrappers are stored in the namespace's bin directory, for example
/// 
/// ```sh
/// $ # this will output way more data, this is for illustrative purposes.
/// $ cat /bin/busybox
/// #!/bin/sh
/// /pkg/store/busybox/1.0.0/bin/busybox
/// ```
///
/// This function is not atomic, failures may leave the system in a broken state.
///
/// Arguments:
/// * `namespace` - The namespace the package was installed in.
/// * `manifest` - The package's manifest.
/// * `installation` - The absolute path to the directory where the package was installed.
async fn expose_bins(
    namespace: &Namespace,
    manifest: &Manifest,
    installation: impl AsRef<Path>,
) -> anyhow::Result<()> {
    for bin in &manifest.exposes.bin {
        let mut wrapper = Wrapper::build(
            namespace.path().display().to_string(),
            &manifest.package.name,
            manifest.package.version.clone(),
            installation
                .as_ref()
                .join("bin")
                .join(&bin.path)
                .display()
                .to_string(),
        )
        .with_declared_variable("NYE_INSTALLATION", installation.as_ref().display().to_string());

        for var in &manifest.consumes.env {
            match var {
                ManifestConsumesEnv::List { name, separator } => {
                    wrapper.add_consumed_variable(name, separator)
                }
                ManifestConsumesEnv::Value { name, value } => {
                    wrapper.add_declared_variable(name, value)
                }
            };
        }

        let mut script = wrapper
            .build()
            .render()
            .context("Could not render binary wrapper into shell script.")?;

        // Clean up blank lines.
        loop {
            if script.replace("\n\n\n", "\n\n") != script {
                script = script.replace("\n\n\n", "\n\n");
            } else {
                break;
            }
        }

        let path = namespace.path().join("bin").join(&bin.link);
        fs::write(&path, script)
            .await
            .context("Could not write binary wrapper contents to bin/ file.")?;
        fs::set_permissions(&path, Permissions::from_mode(0o555))
            .await
            .context("Could not set exposed bin's permissions.")?;
    }

    Ok(())
}

/// Exposes an installed package's libraries.
///
/// This function creates symlinks to the original library files in the namespace's lib directory. For example,
///
/// ```text
/// /lib/libopenssl.so   -> /pkg/store/openssl/1.0.0/lib/libopenssl.so
/// /lib/libopenssl.so.1 -> /pkg/store/openssl/1.0.0/lib/libopenssl.so
/// ```
///
/// This function does not check for collisions, so it's on the caller to ensure none occur. It's
/// not atomic, failures may leave the system in a broken state.
///
/// Arguments:
/// * `namespace` - The namespace the package was installed in.
/// * `manifest` - The package's manifest.
/// * `installation` - The absolute path to the directory where the package was installed.
async fn expose_libs(
    namespace: &Namespace,
    manifest: &Manifest,
    installation: impl AsRef<Path>,
) -> anyhow::Result<()> {
    for lib in &manifest.exposes.lib {
        let link = namespace.path().join("lib").join(&lib.link);
        let source = installation.as_ref().join("lib").join(&lib.path);

        fs::symlink(source, link)
            .await
            .context("Could not create symlink for library.")?;
    }

    Ok(())
}

/// Exposes an installed package's exposed environment variables.
///
/// This function creates individual files with the exposed variables' values. These directories
/// are then symlinked to the namespace's env directory. For example,
///
/// ```text
/// Link:
/// /env/PYTHONPATH/python3-requests/1.0.0 -> /pkg/store/python3-requests/1.0.0/env/PYTHONPATH
/// 
/// Files exposed:
/// /pkg/store/python3-requests/1.0.0/env/PYTHONPATH/0.txt
/// /pkg/store/python3-requests/1.0.0/env/PYTHONPATH/1.txt
/// /pkg/store/python3-requests/1.0.0/env/PYTHONPATH/2.txt
/// ```
/// 
/// The values get the `NYE_INSTALLATION` env variable expanded before storing the value, so that
/// other packages cannot change the way it's interpreted depending on the context.
///
/// This function is not atomic, failures may leave the system in a broken state.
///
/// Arguments:
/// * `namespace` - The namespace the package was installed in.
/// * `manifest` - The package's manifest.
/// * `installation` - The absolute path to the directory where the package was installed.
async fn expose_envs(
    namespace: &Namespace,
    manifest: &Manifest,
    installation: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let mut counters = HashMap::new();
    for var in &manifest.exposes.env {
        let counter = counters.entry(&var.name).or_insert(0);
        let file_path = installation
            .as_ref()
            .join("env")
            .join(&var.name)
            .join(format!("{counter}.txt"));
        let file_path_parent = file_path
            .parent()
            .context("Env value file path did not have a parent??")?;

        fs::create_dir_all(file_path_parent).await.context(
            "Could not ensure parent directories of environment variable value file existed.",
        )?;

        let value = shellexpand::env_with_context_no_errors(&var.value, |s| match s {
            "NYE_INSTALLATION" => Some(installation.as_ref().display().to_string()),
            _ => None,
        })
        .into_owned();
        fs::write(file_path, value)
            .await
            .context("Could not write env var value file.")?;

        if *counter == 0 {
            let source = installation.as_ref().join("env").join(&var.name);
            let link = namespace
                .path()
                .join("env")
                .join(&var.name)
                .join(&manifest.package.name)
                .join(manifest.package.version.to_string());
            let link_parent = link
                .parent()
                .context("Env var link path didn't have any parents???")?;

            fs::create_dir_all(link_parent)
                .await
                .context("Could not ensure env var link's parent directories existed.")?;
            fs::symlink(source, link)
                .await
                .context("Could not link to package's env vars dir.")?;
        }

        *counter += 1;
    }

    Ok(())
}

async fn update_state(
    namespace: &Namespace,
    manifest: &Manifest,
    installation: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let mut connection = namespace
        .state()
        .connection()
        .await
        .context("Could not get a connection to the namespace's state database.")?;
    let mut transaction = connection
        .transaction()
        .await
        .context("Could not get transaction from namespace database connection.")?;

    // TODO: use get or create.
    let package = toasty::create!(Package {
        name: &manifest.package.name,
        created_at: time::utc_now_ms(),
        updated_at: time::utc_now_ms(),
    })
    .exec(&mut transaction)
    .await
    .context("Could not store package in namespace's state database.")?;

    let version = toasty::create!(Version {
        number: manifest.package.version.to_string(),
        package: &package,
        created_at: time::utc_now_ms(),
        updated_at: time::utc_now_ms(),
    })
    .exec(&mut transaction)
    .await
    .context("Could not store package version in namespace's state database.")?;

    update_state_for_artifacts(
        &package,
        &version,
        &manifest.exposes.bin,
        ArtifactKind::Bin,
        &installation,
        &mut transaction,
    )
    .await
    .context("Could not update namespace's database with exposed binaries.")?;
    update_state_for_artifacts(
        &package,
        &version,
        &manifest.exposes.lib,
        ArtifactKind::Lib,
        &installation,
        &mut transaction,
    )
    .await
    .context("Could not update namespace's database with exposed binaries.")?;

    transaction
        .commit()
        .await
        .context("Could not commit transaction to update namespace's state.")?;

    Ok(())
}

async fn update_state_for_artifacts(
    package: &Package,
    version: &Version,
    artifacts: &[ManifestExposesArtifact],
    artifact_kind: ArtifactKind,
    installation: impl AsRef<Path>,
    executor: &mut dyn Executor,
) -> anyhow::Result<()> {
    for artifact in artifacts {
        toasty::create!(Artifact {
            package: package,
            version: version,
            path: installation
                .as_ref()
                .join(artifact_kind.to_string())
                .join(&artifact.path)
                .display()
                .to_string(),
            kind: artifact_kind,
            link: &artifact.link,
            created_at: time::utc_now_ms(),
            updated_at: time::utc_now_ms(),
        })
        .exec(executor)
        .await
        .context("Could not store package's exposed artifacts in namespace's state database.")?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use nye_packages::format::reading::NyeFileSeekableReader;
    use nye_packages::format::reading::safety::Safety;
    use tokio::fs::File;

    use crate::Namespace;
    use crate::installations::Installation;

    #[tokio::test]
    async fn test_install_while_shrinking() -> anyhow::Result<()> {
        let file = File::open("package.nye").await?;
        let file_copy = file.try_clone().await?;
        let mut file_size = file.metadata().await?.len();
        let reader = NyeFileSeekableReader::open(file_copy, Safety::default()).await?;

        let namespace = Namespace::get_for_current_user().await?;
        let installer = Installation::build(namespace, reader);

        installer.validate().await?;

        let _installation = installer
            .install_while_shrinking(async |amount| {
                file_size -= amount;
                file.set_len(file_size).await
            })
            .await?;

        Ok(())
    }
}
