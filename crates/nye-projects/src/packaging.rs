//! Package a nye project into a package file.
//!
//! This module adds packaging-specific associated functions for the types in the [`crate`] and
//! [`crate::manifest`] modules. It gives most of the manifest types an `into_package_version`
//! function that converts the manifest from a project-typed one to a package-typed one.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Ok};
use nye_packages::format::writing::{NyeFileWriteableEntry, NyeFileWriter, Writeable};
use nye_packages::format::{NyeFileEntryKind, Segments};
use nye_packages::manifest as packages;
use nye_schemas::targets::Target;
use tokio::fs::{self, File};
use tokio::sync::mpsc;

use crate::{self as projects, Project, TargetOrShared};

impl projects::Manifest {
    /// Converts a [`projects::Manifest`] into a [`packages::Manifest`] for a specific target.
    pub fn into_package_version(self, target: Target) -> packages::Manifest {
        packages::Manifest {
            package: self.package.into_package_version(target),
            exposes: self.exposes.into_package_version(target),
            consumes: self.consumes.into_package_version(target),
        }
    }
}

impl projects::ManifestPackage {
    /// Converts a [`projects::ManifestPackage`] into a [`packages::ManifestPackage`] for a specific
    /// target.
    pub fn into_package_version(self, target: Target) -> packages::ManifestPackage {
        packages::ManifestPackage {
            name: self.name,
            version: self.version,
            target,
        }
    }
}

impl projects::ManifestExposes {
    /// Converts a [`projects::ManifestExposes`] into a [`packages::ManifestExposes`] for a specific
    /// target.
    pub fn into_package_version(self, target: Target) -> packages::ManifestExposes {
        let bin = collect_artifacts_for_target(self.bin, target);
        let lib = collect_artifacts_for_target(self.lib, target);
        let env = self
            .env
            .into_iter()
            .map(|e| packages::ManifestExposesEnv {
                name: e.name,
                value: e.value,
            })
            .collect();

        packages::ManifestExposes { bin, lib, env }
    }
}

fn collect_artifacts_for_target(
    artifacts: Vec<projects::ManifestExposesArtifact>,
    target: Target,
) -> Vec<packages::ManifestExposesArtifact> {
    let artifacts: Vec<Vec<_>> = artifacts
        .into_iter()
        .filter(|b| b.targets.is_empty() || b.targets.contains(&target))
        .map(|b| {
            b.links
                .into_iter()
                .map(|l| packages::ManifestExposesArtifact {
                    link: l,
                    path: b.path.clone(),
                })
                .collect()
        })
        .collect();
    artifacts.into_iter().flatten().collect()
}

impl projects::ManifestConsumes {
    /// Converts a [`projects::ManifestConsumes`] into a [`packages::ManifestConsumes`] for a
    /// specific target.
    pub fn into_package_version(self, target: Target) -> packages::ManifestConsumes {
        let env = collect_consumed_env(self.env, target);

        packages::ManifestConsumes { env }
    }
}

fn collect_consumed_env(
    env: Vec<projects::ManifestConsumesEnv>,
    target: Target,
) -> Vec<packages::ManifestConsumesEnv> {
    env.into_iter()
        .filter(|e| e.targets().is_empty() || e.targets().contains(&target))
        .map(|e| match e {
            projects::ManifestConsumesEnv::List {
                name,
                separator,
                targets: _,
            } => packages::ManifestConsumesEnv::List { name, separator },
            projects::ManifestConsumesEnv::Value {
                name,
                value,
                targets: _,
            } => packages::ManifestConsumesEnv::Value { name, value },
        })
        .collect()
}

impl Project {
    /// Create a package out of the current project for a specific target.
    ///
    /// A failure of this function does not guarantee the output has not been written to.
    ///
    /// The output file will be created at `project/dist/{name}-v{version}-for-{target}.nye`. The
    /// full path is returned by this function for easy access. It is not recommended to fill up
    /// the output file template yourself, as it may change in future versions.
    ///
    /// This function has other variants:
    /// * [`Project::package_into_file()`] - Write the output file to a specific [`Writeable`] (e.g.
    ///   a [`File`]).
    /// * [`Project::package_with_progress()`] - Same as this function plus an event handler to keep
    ///   track of progress.
    /// * [`Project::package_into_file_with_progress()`] - A combination of the two above.
    ///
    /// Arguments:
    /// * `target` - The target to build the package file for.
    ///
    /// Returns:
    /// * `Ok(PathBuf)` - The path to the output file, if successfully built.
    /// * `Err(Error)` - If an error occurred while building the package file.
    pub async fn package(&self, target: Target) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(self.path.join("dist"))
            .await
            .context("Could not ensure the dist directory existed at the project's root.")?;

        let output_file_path = self.path.join("dist").join(format!(
            "{}-v{}-for-{}.nye",
            self.manifest.package.name, self.manifest.package.version, target
        ));
        let output_file = File::create(&output_file_path)
            .await
            .context("Could not open output file.")?;

        self.package_into_file_with_progress(target, output_file, || {})
            .await
            .map(|_| output_file_path)
    }

    /// Create a package out of the current project for a specific target and write it to the
    /// specified output.
    ///
    /// A failure of this function does not guarantee the output has not been written to.
    ///
    /// This function has other variants:
    /// * [`Project::package()`] - Write the output file to the default output path in the project's
    ///   dist directory.
    /// * [`Project::package_with_progress()`] - Write the output file to the default output path in
    ///   the project's dist directory plus an event handler to track progress.
    /// * [`Project::package_into_file_with_progress()`] - A combination of this function and the
    ///   one above.
    ///
    /// Arguments:
    /// * `target` - The target to build the package file for.
    ///
    /// Returns:
    /// * `Ok(())` - If the package file is successfully built.
    /// * `Err(Error)` - If an error occurred while building the package file.
    pub async fn package_into_file(
        &self,
        target: Target,
        file: impl Writeable,
    ) -> anyhow::Result<()> {
        self.package_into_file_with_progress(target, file, || {})
            .await
    }

    /// Create a package out of the current project for a specific target and handle progress
    /// events.
    ///
    /// A failure of this function does not guarantee the output has not been written to.
    ///
    /// The output file will be created at `project/dist/{name}-v{version}-for-{target}.nye`. The
    /// full path is returned by this function for easy access. It is not recommended to fill up
    /// the output file template yourself, as it may change in future versions.
    /// 
    /// The progress handler can take a few different function types as handlers:
    /// * `|| {}`
    /// * `|| { Ok(()) }`
    /// * `|event: ProgressEvent| {}`
    /// * `|event: ProgressEvent| { Ok(()) }`
    ///
    /// This function has other variants:
    /// * [`Project::package()`] - Write the output to the default output in the project's dist
    ///   folder, without progress tracking.
    /// * [`Project::package_into_file()`] - Write the output file to a specific [`Writeable`] (e.g.
    ///   a [`File`]).
    /// * [`Project::package_into_file_with_progress()`] - A combination of this function and the
    ///   one above.
    ///
    /// Arguments:
    /// * `target` - The target to build the package file for.
    ///
    /// Returns:
    /// * `Ok(PathBuf)` - The path to the output file, if successfully built.
    /// * `Err(Error)` - If an error occurred while building the package file.
    pub async fn package_with_progress<Args, Return>(
        &self,
        target: Target,
        progress: impl Progress<Args, Return> + Send + 'static,
    ) -> anyhow::Result<PathBuf> {
        fs::create_dir_all(self.path.join("dist"))
            .await
            .context("Could not ensure the dist directory existed at the project's root.")?;

        let output_file_path = self.path.join("dist").join(format!(
            "{}-v{}-for-{}.nye",
            self.manifest.package.name, self.manifest.package.version, target
        ));
        let output_file = File::create(&output_file_path)
            .await
            .context("Could not open output file.")?;

        self.package_into_file_with_progress(target, output_file, progress)
            .await
            .map(|_| output_file_path)
    }

    /// Create a package out of the current project for a specific target, writing its progress to
    /// the specified outputs and handling progress events.
    ///
    /// A failure of this function does not guarantee the output has not been written to.
    /// 
    /// The progress handler can take a few different function types as handlers:
    /// * `|| {}`
    /// * `|| { Ok(()) }`
    /// * `|event: ProgressEvent| {}`
    /// * `|event: ProgressEvent| { Ok(()) }`
    ///
    /// This function has other variants:
    /// * [`Project::package()`] - Write the output file to the default output path in the project's
    ///   dist directory.
    /// * [`Project::package_into_file()`] - Write the output file to the specified output.
    /// * [`Project::package_with_progress()`] - Write the output file to the default output path in
    ///   the project's dist directory and handle progress events.
    ///
    /// Arguments:
    /// * `target` - The target to build the package file for.
    ///
    /// Returns:
    /// * `Ok(())` - If the package file is successfully built.
    /// * `Err(Error)` - If an error occurred while building the package file.
    pub async fn package_into_file_with_progress<Args, Return>(
        &self,
        target: Target,
        file: impl Writeable,
        mut progress: impl Progress<Args, Return> + Send + 'static,
    ) -> anyhow::Result<()> {
        let manifest = self.manifest.clone().into_package_version(target);
        let mut writer = NyeFileWriter::new(manifest);

        let target_source = self
            .manifest
            .targets
            .get(&TargetOrShared::Target(target))
            .context("The specified target was not configured.")?
            .source
            .clone();
        let shared_source = self
            .manifest
            .targets
            .get(&TargetOrShared::Shared)
            .map(|i| i.source.clone());

        // We discard collisions later on. For that reason, the shared source has to go after the
        // target source (target-specific files take precedence over shared ones).
        //
        // This vector holds (source_root, file_kind, file_name_in_source) for easier decoupling
        // down in this function.
        let mut pending = vec![
            (target_source.clone(), NyeFileEntryKind::Bin, PathBuf::new()),
            (target_source.clone(), NyeFileEntryKind::Lib, PathBuf::new()),
            (target_source.clone(), NyeFileEntryKind::Etc, PathBuf::new()),
            (target_source.clone(), NyeFileEntryKind::Var, PathBuf::new()),
        ];
        if let Some(shared_source) = shared_source {
            pending.extend([
                (shared_source.clone(), NyeFileEntryKind::Bin, PathBuf::new()),
                (shared_source.clone(), NyeFileEntryKind::Lib, PathBuf::new()),
                (shared_source.clone(), NyeFileEntryKind::Etc, PathBuf::new()),
                (shared_source.clone(), NyeFileEntryKind::Var, PathBuf::new()),
            ]);
        }

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let event_forwarder = tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                progress.progress(event).await?;
            }

            Ok(())
        });

        let mut total = 0;
        let loaded = Arc::new(AtomicUsize::new(0));

        for i in 0.. {
            let (root, kind, name) = match pending.get(i) {
                Some(path) => path.clone(),
                None => break,
            };

            let path = self.path.join(&root).join(kind.to_string()).join(&name);
            if !fs::try_exists(&path)
                .await
                .context("Could not check if pending directory existed.")?
            {
                continue;
            }

            let mut readdir = fs::read_dir(&path)
                .await
                .context("Could not read directory in target source.")?;
            while let Some(entry) = readdir
                .next_entry()
                .await
                .context("Could not read next entry in target source.")?
            {
                let file_type = entry
                    .file_type()
                    .await
                    .context("Could not get file type of entry in target source.")?;

                if file_type.is_symlink() {
                    continue;
                } else if file_type.is_dir() {
                    let ext = path
                        .file_name()
                        .context("An entry in target source directory was unnamed.")?;
                    pending.push((root.clone(), kind, name.join(ext)));
                } else {
                    let entry_path = entry.path();
                    let entry_name = Segments::from_str(&name.display().to_string())
                        .context("context")
                        .context("File in target source had an invalid name.")?;

                    let loaded = loaded.clone();
                    let event_tx_copy = event_tx.clone();
                    let entry = NyeFileWriteableEntry::new(
                        kind,
                        name.display().to_string(),
                        async move || {
                            let _ = event_tx_copy.send(ProgressEvent::Loading {
                                index: loaded.fetch_add(1, Ordering::SeqCst),
                                total,
                                kind,
                                name: entry_name,
                            });
                            Ok(File::open(entry_path).await?)
                        },
                    )
                    .context("File in target source had an invalid name.")?;

                    // Will only fail if colliding. Since only collision chances are when loading
                    // the shared artifacts, shared artifacts are loaded last (see `pending`
                    // initial value ordering) and we discard them on collision.
                    if writer.insert(entry).is_ok() {
                        // By this time, the total is being incremented once with every entry. It's
                        // not the actual total yet.
                        let _ = event_tx.send(ProgressEvent::Scanning { index: total });
                        total += 1;
                    }
                }
            }
        }

        writer
            .write(file)
            .await
            .context("Could not write the package file.")?;

        let _ = event_tx.send(ProgressEvent::Done { total });
        event_forwarder
            .await
            .context("The event-forwarding task panicked.")?
            .context("The progress event handler failed.")?;

        Ok(())
    }
}

/// A progress update event.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// A new entry was scanned.
    Scanning {
        /// The index of the file loaded.
        ///
        /// This is incremented by one with each event of this type. This does not equal to an
        /// artifact's position in a package file directory.
        index: usize,
    },

    /// An entry is being loaded.
    Loading {
        /// The index of the file being loaded.
        index: usize,
        /// The total amount of entries to load.
        total: usize,
        /// The kind of entry being loaded.
        kind: NyeFileEntryKind,
        /// The name of the entry being loaded.
        name: Segments,
    },

    /// All entries have been successfully loaded.
    Done {
        /// The total amount of entries loaded.
        total: usize,
    },
}

/// Trait implemented by progress event handlers.
pub trait Progress<Args, Return> {
    /// Called when a progress event occurs.
    fn progress(
        &mut self,
        event: ProgressEvent,
    ) -> impl Future<Output = anyhow::Result<()>> + Send + Sync;
}

impl<F> Progress<(), ()> for F
where
    F: FnMut() + Send + Sync,
{
    async fn progress(&mut self, _event: ProgressEvent) -> anyhow::Result<()> {
        self();
        Ok(())
    }
}

impl<F, E> Progress<(), anyhow::Result<()>> for F
where
    F: FnMut() -> Result<(), E> + Send + Sync,
    E: Into<anyhow::Error>,
{
    async fn progress(&mut self, _event: ProgressEvent) -> anyhow::Result<()> {
        self().map_err(Into::into)
    }
}

impl<F> Progress<(ProgressEvent,), ()> for F
where
    F: FnMut(ProgressEvent) + Send + Sync,
{
    async fn progress(&mut self, event: ProgressEvent) -> anyhow::Result<()> {
        self(event);
        Ok(())
    }
}

impl<F, E> Progress<(ProgressEvent,), anyhow::Result<()>> for F
where
    F: FnMut(ProgressEvent) -> Result<(), E> + Send + Sync,
    E: Into<anyhow::Error>,
{
    async fn progress(&mut self, event: ProgressEvent) -> anyhow::Result<()> {
        self(event).map_err(Into::into)
    }
}
