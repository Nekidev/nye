//! A crate to manage nye package projects.
//!
//! This crate is internally used by `nye` for the `nye dev` commands. It is, however, meant to be
//! used as a standalone library other people can use, and is structured as such.
//!
//! # Installation
//!
//! To add this crate to another `nye` crate, run the following command inside its crate directory:
//!
//! ```sh
//! $ cargo add nye-projects
//! ```
//!
//! # Usage
//!
//! This crate provides functionality both to initialize projects, to get the current project, and
//! to package projects into package files.
//!
//! ## Create a Project
//!
//! Creating a project is simple. You can use [`ProjectBuilder`] to define what will be created,
//! then use [`ProjectBuilder::init()`] to create it in the local filesystem.
//!
//! ```
//! let builder = PackageBuilder::new("my-project")?
//!     .with_target(TargetOrShared::Shared)
//!     .with_target(Target::get_current()?);
//! let project = builder.init(".").await?;
//! ```
//!
//! ## Get the Current Project
//!
//! To get the project the current working directory belongs to, use [`Project::get_current()`].
//! It'll search for a project manifest file (`nye.toml`) in the current directory and every parent
//! directory.
//! 
//! ```
//! let project = match Project::get_current()? {
//!     Some(project) => project,
//!     None => anyhow::bail!("You're not inside a project!"),
//! };
//! ```
//! 
//! You can then access the following attributes of the project:
//! 
//! - [`Project::path`] - The project's root directory.
//! - [`Project::manifest`] - The project manifest's contents.

use std::env;
use std::path::PathBuf;

use anyhow::Context;
use tokio::fs;

pub mod builder;
pub mod manifest;

pub use builder::ProjectBuilder;
pub use manifest::{
    Manifest, ManifestConsumes, ManifestConsumesEnv, ManifestExposes, ManifestExposesArtifact,
    ManifestExposesEnv, ManifestPackage, ManifestTarget, TargetOrShared,
};

/// A project in the local filesystem.
#[derive(Clone)]
pub struct Project {
    /// The root path to the project's directory.
    pub path: PathBuf,
    /// The project's manifest.
    pub manifest: Manifest,
}

impl Project {
    /// Gets the project the current working directory belongs to, if any.
    ///
    /// Returns:
    /// * `Ok(Some(Project))` - The project in the current working directory.
    /// * `Ok(None)` - The current working directory doesn't belong to any project.
    /// * `Err(Error)` - An error occurred while getting the project the current working directory
    ///   belongs to.
    pub async fn get_current() -> anyhow::Result<Option<Self>> {
        let mut current_path =
            env::current_dir().context("Could not get the current working directory.")?;
        loop {
            let manifest_path = current_path.join("nye.toml");

            if fs::try_exists(&manifest_path)
                .await
                .context("Could not check if the manifest existed.")?
            {
                let manifest = manifest::load(manifest_path)
                    .await
                    .context("Could not load the project's manifest file.")?;

                return Ok(Some(Project {
                    path: current_path,
                    manifest,
                }));
            }

            if let Some(parent) = current_path.parent() {
                current_path = parent.to_path_buf();
            } else {
                break;
            }
        }

        Ok(None)
    }
}
