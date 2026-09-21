//! Nye project builder.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use nye_schemas::semver::Semver;
use tokio::fs;

use crate::Project;
use crate::manifest::{
    Manifest, ManifestConsumes, ManifestExposes, ManifestPackage, ManifestTarget, TargetOrShared,
};

/// The default project .gitignore contents.
const GITIGNORE: &str = dedent::dedent!(
    "
    /dist
    "
);

/// A nye project builder.
///
/// Use it to initialize new projects in the local filesystem.
///
/// For example,
///
/// ```
/// let builder = ProjectBuilder::new("example")?
///     .with_target(TargetOrShared::Shared)
///     .with_target(Target::get_current()?);
/// ```
#[derive(Debug, Clone)]
pub struct ProjectBuilder {
    name: String,
    version: Semver,
    targets: HashSet<TargetOrShared>,
}

impl ProjectBuilder {
    /// Creates a new project builder.
    ///
    /// The passed project name must follow these rules:
    ///
    /// * Must be kebab case.
    /// * Must not start with `-`.
    /// * Must be a safe path component.
    ///
    /// You can use [`nye_validation::is_kebab_case`] and [`nye_validation::is_safe_path_component`]
    /// to validate it, as they're the same checks this function uses internally.
    ///
    /// Arguments:
    /// * `name` - The name to initialize the project as.
    pub fn new(name: impl Into<String>) -> anyhow::Result<Self> {
        let name = name.into();

        nye_validation::is_kebab_case(&name).context("The project's name must be kebab case.")?;
        nye_validation::is_safe_path_component(&name)
            .context("The project's name must be a safe path component.")?;

        Ok(Self {
            name,
            version: Semver::from_str("0.0.0").unwrap(),
            targets: HashSet::new(),
        })
    }

    /// Adds a target to the project.
    ///
    /// The values accepted by this function are either:
    ///
    /// * [`Target`]
    /// * [`TargetOrShared`]
    ///
    /// Arguments:
    /// * `target` - The target to add to the project builder.
    ///
    /// [`Target`]: nye_schemas::targets::Target
    pub fn with_target(mut self, target: impl Into<TargetOrShared>) -> Self {
        self.targets.insert(target.into());
        self
    }

    /// Initializes the built project in the specified path.
    ///
    /// This method creates all the default project files and manifest.
    ///
    /// Arguments:
    /// * `path` - The directory where the project will be initialized. It must exist and not
    ///   collision with the new directories and files.
    pub async fn init(self, path: impl AsRef<Path>) -> anyhow::Result<Project> {
        let path = path.as_ref();

        self.init_dirs(path)
            .await
            .context("Could not initialize project directories.")?;
        self.init_manifest(path)
            .await
            .context("Could not initialize project's manifest.")?;
        self.init_gitignore(path)
            .await
            .context("Could not initialize project's .gitignore.")?;

        Ok(Project {
            path: path.to_path_buf(),
            manifest: self.build_manifest(),
        })
    }

    /// Generates the manifest file for this project.
    fn build_manifest(&self) -> Manifest {
        let package = ManifestPackage {
            name: self.name.clone(),
            version: self.version.clone(),
        };

        let mut targets = HashMap::new();
        for target in &self.targets {
            targets.insert(
                *target,
                ManifestTarget {
                    source: PathBuf::from(format!("src/{target}")),
                },
            );
        }

        Manifest {
            package,
            targets,
            exposes: ManifestExposes::default(),
            consumes: ManifestConsumes::default(),
        }
    }

    /// Initializes the default directories in the project directory.
    ///
    /// For every configured target, this function initializes the following directories:
    ///
    /// * `{target.source}/bin`
    /// * `{target.source}/lib`
    /// * `{target.source}/etc`
    /// * `{target.source}/var`
    ///
    /// Only the target source directories are created. If the project's root directory does not
    /// exist, this function will fail. This function will also fail if the directories already
    /// exist.
    ///
    /// Arguments:
    /// * `path` - The project's root directory.
    async fn init_dirs(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let manifest = self.build_manifest();

        let path = path.as_ref();

        for (_target, config) in manifest.targets {
            let mut path = path.to_path_buf();
            for component in config.source.components() {
                path = path.join(component);
                fs::create_dir(&path)
                    .await
                    .context("Could not create directory inside project directory.")?;
            }

            let dirs = [
                path.join("bin"),
                path.join("lib"),
                path.join("etc"),
                path.join("var"),
            ];

            for dir in dirs {
                fs::create_dir(dir).await.context(
                    "Could not create artifact directory inside target source directory.",
                )?;
            }
        }

        Ok(())
    }

    /// Creates the manifest file in the project's root directory.
    ///
    /// Arguments:
    /// * `path` - The project's root directory.
    async fn init_manifest(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let manifest = self.build_manifest();
        let manifest_string = toml::to_string_pretty(&manifest)
            .context("Could not serialize manifest into TOML string.")?;

        fs::write(path.as_ref().join("nye.toml"), manifest_string)
            .await
            .context("Could not write manifest to file.")?;

        Ok(())
    }

    /// Creates the default .gitignore file in the project's root directory.
    ///
    /// The default .gitignore's contents can be found in [`GITIGNORE`].
    ///
    /// Arguments:
    /// * `path` - The project's root directory.
    async fn init_gitignore(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        fs::write(path.as_ref().join(".gitignore"), GITIGNORE)
            .await
            .context("Could not write the default .gitignore's contents to .gitignore inside the project's root directory.")?;

        Ok(())
    }
}
