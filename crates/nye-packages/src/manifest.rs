//! Package manifest schemas. A package file's "configuration".
//!
//! # Comparison with Project Manifests
//!
//! This manifest type reensembles package project manifests. However, these are aimed at
//! explicitness and single-target configurations.
//!
//! For example, a project manifest has `targets.*` configurations and things like `shared`, while
//! package manifests have a single `package.target` instead. Artifacts exposed under multiple
//! links are expanded into one item per link.
//! 
//! Package manifests are not meant to be written by humans. They may be read by humans, though, so
//! they're serialized into prettified TOML when creating package files. They are held in a special
//! section within a package file, not as a regular entry.
//! 
//! # Learn by Example
//! 
//! As stated above, package manifests are aimed at a single target, which is specified in the
//! `[package]` section.
//! 
//! ```toml
//! version = 1
//! 
//! [package]
//! name = "example"
//! version = "1.1.1"
//! target = "linux-x86_64"
//! ```
//! 
//! Exposed and consumed resources are defined in a similar way to in project manifests.
//! 
//! ```toml
//! [exposes.bin]
//! link = "example"
//! path = "example"
//! 
//! [exposes.lib]
//! link = "example.so"
//! path = "example.so"
//! ```
//! 
//! Unlike project manifests, multiple links cannot be defined as an array, and both `link` and
//! `path` are required. Since package files are single-target, no `targets` field exists.

use std::path::PathBuf;

use nye_schemas::semver::Semver;
use nye_schemas::targets::Target;
use serde::{Deserialize, Serialize};

/// A package's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// The package's metadata.
    ///
    /// Contains information such as the project's name, version, and target.
    pub package: ManifestPackage,
    /// The package's exposed artifacts.
    #[serde(skip_serializing_if = "ManifestExposes::is_empty")]
    pub exposes: ManifestExposes,
    /// The package's consumed artifacts and configurations.
    #[serde(skip_serializing_if = "ManifestConsumes::is_empty")]
    pub consumes: ManifestConsumes,
}

/// A package's metadata.
///
/// Contains information such as the project's name, version, and target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPackage {
    /// The package's name.
    pub name: String,
    /// The package's version.
    pub version: Semver,
    /// The target this package was built for.
    ///
    /// Installing this package on different targets will very likely (99% of cases) install a
    /// broken package, since binaries, libraries, and other artifacts will have been built for the
    /// specified target.
    pub target: Target,
}

/// Exposed artifacts and env vars by a package.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestExposes {
    /// The package's exposed binaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<ManifestExposesArtifact>,

    /// The package's exposed libraries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib: Vec<ManifestExposesArtifact>,

    /// The package's exposed environment variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<ManifestExposesEnv>,
}

impl ManifestExposes {
    /// `true` when all [`ManifestExposes::bin`], [`ManifestExposes::lib`], and
    /// [`ManifestExposes::env`] are empty.
    fn is_empty(&self) -> bool {
        self.bin.is_empty() && self.lib.is_empty() && self.env.is_empty()
    }
}

/// An exposed artifact (binary, library, configuration file) by the package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestExposesArtifact {
    /// The name under which this artifact will be exposed.
    ///
    /// It must follow the following rules:
    /// * Be a single safe path segment,
    /// * Be at least 1 character long,
    /// * Be no longer than 32 characters long.
    pub link: String,
    /// The path to the artifact's file within its corresponding directory.
    ///
    /// For example, for a `bin/busybox` binary, this is `busybox`. The `bin/` prefix is
    /// automatically added based on the artifact kind.
    pub path: PathBuf,
}

/// An exposed environment variable value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestExposesEnv {
    /// The name of the environment variable.
    ///
    /// It must follow the following rules:
    /// * Be at least 1 character long,
    /// * Be no more than 32 characters long,
    /// * Not be `NYE_INSTALLATION`,
    /// * Be composed of only ASCII letters, digits, and underscores.
    pub name: String,
    /// The value exposed by the environment variable.
    ///
    /// It must not be longer than 512 characters.
    pub value: String,
}

/// A package's consumed environment variables.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestConsumes {
    /// The consumed environment variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<ManifestConsumesEnv>,
}

impl ManifestConsumes {
    pub fn is_empty(&self) -> bool {
        self.env.is_empty()
    }
}

/// A consumed environment variable's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManifestConsumesEnv {
    /// An environment variable consumed from a single string.
    ///
    /// The string may contain references to other environment variables, which will be expanded
    /// at runtime. `NYE_INSTALLATION` is expanded and hardcoded to the value on installation to
    /// keep it pinned to the installed package-specific value.
    Value { name: String, value: String },
    /// An environment variable consumed from other packages' exposed variable values.
    ///
    /// The separator is used to chain each package's value. For example, for the example packages
    /// * `python3-requests`
    /// * `python3-uvicorn`
    /// * `python3`
    ///
    /// The packages `python3-requests` and `python3-uvicorn` may expose `PYTHONPATH` set to
    /// `${NYE_INSTALLATION}/lib`. Then, when the `python3` package consumes `PYTHONPATH` using `:`
    /// as the separator, it'll see the following value
    ///
    /// ```text
    /// /pkg/store/python3-requests/1.0.0/lib:/pkg/store/python3-uvicorn/1.0.0/lib
    /// ```
    ///
    /// The actual value will depend on the real installation path of each package, the value above
    /// is for illustrative purposes.
    List { name: String, separator: String },
}

impl ManifestConsumesEnv {
    pub fn name(&self) -> &String {
        match &self {
            ManifestConsumesEnv::List { name, .. } => name,
            ManifestConsumesEnv::Value { name, .. } => name,
        }
    }
}
