use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use regex::Regex;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};

use crate::semver::Semver;
use crate::targets::Target;
use crate::validation::{self, Validate};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetOrShared {
    Shared,
    Target(Target),
}

impl Display for TargetOrShared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetOrShared::Shared => write!(f, "shared"),
            TargetOrShared::Target(target) => target.fmt(f),
        }
    }
}

impl Serialize for TargetOrShared {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Shared => serializer.serialize_str("shared"),
            Self::Target(target) => target.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for TargetOrShared {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TargetOrSharedVisitor;

        impl<'de> Visitor<'de> for TargetOrSharedVisitor {
            type Value = TargetOrShared;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A valid system target, e.g. linux-x86_64.")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v == "shared" {
                    return Ok(TargetOrShared::Shared);
                }

                let target = Target::from_str(v).map_err(|_| {
                    E::custom("The specified target was neither a valid target nor `shared`.")
                })?;

                Ok(TargetOrShared::Target(target))
            }
        }

        deserializer.deserialize_str(TargetOrSharedVisitor)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: ManifestPackage,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub targets: HashMap<TargetOrShared, ManifestTarget>,

    #[serde(default, skip_serializing_if = "ManifestExposes::is_empty")]
    pub exposes: ManifestExposes,

    #[serde(default, skip_serializing_if = "ManifestConsumes::is_empty")]
    pub consumes: ManifestConsumes,
}

impl Validate for Manifest {
    fn validate(&self) -> anyhow::Result<()> {
        self.package
            .validate()
            .context("The manifest's `package` field was not valid.")?;

        for (k, v) in &self.targets {
            if let TargetOrShared::Target(target) = k
                && !target.is_supported()
            {
                anyhow::bail!(
                    "The configured target `{target}` is properly formatted but not supported by nye."
                );
            }

            v.validate().context(format!(
                "The configured target `{k}` had an invalid configuration."
            ))?;
        }

        self.exposes
            .validate()
            .context("The manifest's `exposes` field was not valid.")?;
        self.consumes
            .validate()
            .context("The manifest's `consumes` field was not valid.")?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestPackage {
    pub name: String,
    pub version: Semver,
}

impl Validate for ManifestPackage {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=32).contains(&self.name.len()) {
            anyhow::bail!(
                "The package's name must be at least 1 character long and less or equal to 32 characters long."
            );
        }

        validation::is_kebab_case(&self.name)
            .context("The package name was not valid kebab case")?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestTarget {
    pub source: PathBuf,
}

impl Validate for ManifestTarget {
    fn validate(&self) -> anyhow::Result<()> {
        validation::is_safe_path(&self.source).context("The target's source path was not safe.")?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ManifestExposes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<ManifestExposesArtifact>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib: Vec<ManifestExposesArtifact>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<ManifestExposesEnv>,
}

impl ManifestExposes {
    fn is_empty(&self) -> bool {
        self.lib.is_empty() && self.lib.is_empty() && self.env.is_empty()
    }
}

fn validate_artifact_duplicate_links(artifacts: &[ManifestExposesArtifact]) -> anyhow::Result<()> {
    let mut links = HashSet::new();
    for artifact in artifacts {
        artifact.validate().context(format!(
            "The exposed artifact `{}` was incorrectly configured.",
            artifact.path.display()
        ))?;

        for link in &artifact.links {
            if links.contains(link) {
                anyhow::bail!("Two or more exposed artifacts conflict on the linked name `{link}`.")
            }

            links.insert(link);
        }
    }

    Ok(())
}

impl Validate for ManifestExposes {
    fn validate(&self) -> anyhow::Result<()> {
        validate_artifact_duplicate_links(&self.bin)
            .context("One or more exposed binaries are incorrectly configured.")?;
        validate_artifact_duplicate_links(&self.lib)
            .context("One or more exposed libraries are incorrectly configured.")?;

        for env in &self.env {
            env.validate()
                .context("An exposed environment variable was incorrectly configured.")?;
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestExposesArtifact {
    pub path: PathBuf,

    /// When empty, it defaults to the file name.
    #[serde(default)]
    pub links: HashSet<String>,

    /// When empty, it defaults to all targets supported by the package.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub targets: HashSet<Target>,
}

impl Validate for ManifestExposesArtifact {
    fn validate(&self) -> anyhow::Result<()> {
        validation::is_safe_path(&self.path).context(format!(
            "The specified artifact path `{}` is not safe.",
            self.path.display()
        ))?;

        for link in &self.links {
            if !(1..=32).contains(&link.len()) {
                anyhow::bail!(
                    "Linked names must be at least one character long and up to 32 characters long."
                );
            }

            validation::is_safe_path_component(link).context(format!(
                "The specified linked name `{link}` was not a safe path component."
            ))?;
        }

        for target in &self.targets {
            if !target.is_supported() {
                anyhow::bail!(
                    "The specified target `{target}` for exposed artifact is not supported by nye."
                );
            }
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestExposesEnv {
    pub name: String,
    pub value: String,

    /// When empty, it defaults to all targets supported by the package.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub targets: HashSet<Target>,
}

impl Validate for ManifestExposesEnv {
    fn validate(&self) -> anyhow::Result<()> {
        let regex = Regex::new("^[a-zA-Z0-9_]{1,32}$")
            .context("This is a bug. The hard-coded validation regex was invalid.")?;

        if !regex.is_match(&self.name) {
            anyhow::bail!(
                "Environment variable names must only contain lowercase letters, uppercase letters, numbers, and underscores. `{}` did not fit these requirements.",
                self.name
            );
        }

        if &self.name == "NYE_INSTALLATION" {
            anyhow::bail!(
                "NYE_INSTALLATION environment variable cannot be exposed, it's automatically set by nye."
            );
        }

        if self.value.len() > 512 {
            anyhow::bail!(
                "Environment variable exposed values must not exceed 512 bytes in length. `{}` exceeds this limit.",
                self.name
            );
        }

        for target in &self.targets {
            if !target.is_supported() {
                anyhow::bail!(
                    "The specified target `{target}` for exposed environment variable is not supported by nye."
                );
            }
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ManifestConsumes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<ManifestConsumesEnv>,
}

impl ManifestConsumes {
    pub fn is_empty(&self) -> bool {
        self.env.is_empty()
    }
}

impl Validate for ManifestConsumes {
    fn validate(&self) -> anyhow::Result<()> {
        let mut names = HashSet::new();

        for var in &self.env {
            var.validate()
                .context("A declared consumed environment variable was invalid.")?;

            if names.contains(var.name()) {
                anyhow::bail!("You cannot declare a consumed environment variable twice.");
            }

            names.insert(var.name().clone());
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ManifestConsumesEnv {
    Value {
        name: String,
        value: String,

        /// When empty, it defaults to all targets supported by the package.
        #[serde(default, skip_serializing_if = "HashSet::is_empty")]
        targets: HashSet<Target>,
    },
    List {
        name: String,
        separator: String,

        /// When empty, it defaults to all targets supported by the package.
        #[serde(default, skip_serializing_if = "HashSet::is_empty")]
        targets: HashSet<Target>,
    },
}

impl ManifestConsumesEnv {
    pub fn name(&self) -> &String {
        match &self {
            ManifestConsumesEnv::List { name, .. } => name,
            ManifestConsumesEnv::Value { name, .. } => name,
        }
    }

    pub fn targets(&self) -> &HashSet<Target> {
        match &self {
            ManifestConsumesEnv::List { targets, .. } => targets,
            ManifestConsumesEnv::Value { targets, .. } => targets,
        }
    }
}

impl Validate for ManifestConsumesEnv {
    fn validate(&self) -> anyhow::Result<()> {
        let regex = Regex::new("^[a-zA-Z0-9_]{1,32}$")
            .context("This is a bug. The hard-coded validation regex was invalid.")?;

        if !regex.is_match(self.name()) {
            anyhow::bail!(
                "Environment variable names must only contain lowercase letters, uppercase letters, numbers, and underscores. `{}` did not fit these requirements.",
                self.name()
            );
        }

        if self.name().as_str() == "NYE_INSTALLATION" {
            anyhow::bail!(
                "NYE_INSTALLATION environment variable cannot be consumed, it's automatically set by nye."
            );
        }

        for target in self.targets() {
            if !target.is_supported() {
                anyhow::bail!(
                    "The specified target `{target}` for consumed environment variable is not supported by nye."
                );
            }
        }

        match &self {
            ManifestConsumesEnv::List { separator, .. } => {
                if !(0..=16).contains(&separator.len()) {
                    anyhow::bail!(
                        "Consumed environment variables cannot have a separator longer than 16 bytes."
                    );
                }
            }
            ManifestConsumesEnv::Value { value, .. } => {
                if !(0..=512).contains(&value.len()) {
                    anyhow::bail!(
                        "Consumed environment variables cannot have a value longer than 512 bytes."
                    );
                }
            }
        }

        Ok(())
    }
}
