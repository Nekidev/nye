use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::semver::Semver;
use crate::targets::Target;
use crate::validation::Validate;
use crate::{projects, validation};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub package: ManifestPackage,

    #[serde(default, skip_serializing_if = "ManifestExposes::is_empty")]
    pub exposes: ManifestExposes,

    #[serde(default, skip_serializing_if = "ManifestConsumes::is_empty")]
    pub consumes: ManifestConsumes,
}

fn collect_artifacts(
    artifacts: &[projects::manifest::ManifestExposesArtifact],
    target: &Target,
) -> Vec<ManifestExposesArtifact> {
    let mut result = Vec::new();
    for artifact in artifacts {
        if artifact.targets.contains(target) || artifact.targets.is_empty() {
            for link in &artifact.links {
                result.push(ManifestExposesArtifact {
                    link: link.clone(),
                    path: artifact.path.clone(),
                });
            }
        }
    }

    result
}

impl Manifest {
    /// Returns a package manifest from a project manifest.
    ///
    /// This function does not validate the contents of the project manifest, it's on the caller to
    /// ensure the manifest specified is valid. An invalid manifest input will produce an invalid
    /// manifest output.
    ///
    /// Arguments:
    /// * `manifest` - The project's manifest.
    /// * `target` - The target this package manifest is for.
    pub fn from_project_manifest(manifest: projects::Manifest, target: Target) -> Self {
        let exposes_bin = collect_artifacts(&manifest.exposes.bin, &target);
        let exposes_lib = collect_artifacts(&manifest.exposes.lib, &target);
        let exposes_env = {
            let mut result = vec![];

            for var in &manifest.exposes.env {
                if !var.targets.contains(&target) && !var.targets.is_empty() {
                    continue;
                }

                result.push(ManifestExposesEnv {
                    name: var.name.clone(),
                    value: var.value.clone(),
                });
            }

            result
        };

        let consumes_env = {
            let mut result = vec![];

            for var in &manifest.consumes.env {
                if !var.targets().contains(&target) && !var.targets().is_empty() {
                    continue;
                }

                let converted = match var.clone() {
                    projects::manifest::ManifestConsumesEnv::List {
                        name,
                        separator,
                        targets: _,
                    } => ManifestConsumesEnv::List { name, separator },
                    projects::manifest::ManifestConsumesEnv::Value {
                        name,
                        value,
                        targets: _,
                    } => ManifestConsumesEnv::Value { name, value },
                };

                result.push(converted);
            }

            result
        };

        Manifest {
            package: ManifestPackage {
                name: manifest.package.name,
                version: manifest.package.version,
                target,
            },
            exposes: ManifestExposes {
                bin: exposes_bin,
                lib: exposes_lib,
                env: exposes_env,
            },
            consumes: ManifestConsumes { env: consumes_env },
        }
    }
}

impl Validate for Manifest {
    fn validate(&self) -> anyhow::Result<()> {
        self.package
            .validate()
            .context("The package field of the manifest was invalid.")?;
        self.exposes
            .validate()
            .context("The manifest's configured exposed artifacts were invalid.")?;
        self.consumes
            .validate()
            .context("The manifest's configured consumed artifacts were invalid.")?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ManifestPackage {
    pub name: String,
    pub version: Semver,
    pub target: Target,
}

impl Validate for ManifestPackage {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=32).contains(&self.name.len()) {
            anyhow::bail!(concat!(
                "The package's name must be at least 1 character long and less or equal to 32 ",
                "characters long."
            ));
        }

        validation::is_kebab_case(&self.name)
            .context("The package name was not valid kebab case")?;

        if !self.target.is_supported() {
            anyhow::bail!("The configured target in the manifest is not supported by nye.");
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct ManifestExposes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<ManifestExposesArtifact>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib: Vec<ManifestExposesArtifact>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<ManifestExposesEnv>,
}

fn validate_artifacts(artifacts: &[ManifestExposesArtifact]) -> anyhow::Result<()> {
    let mut links = HashSet::new();
    for artifact in artifacts {
        artifact.validate().context(format!(
            "The exposed artifact `{}` was incorrectly configured.",
            artifact.path.display()
        ))?;

        if links.contains(&artifact.link) {
            anyhow::bail!(
                "Two or more exposed artifacts conflict on the linked name `{}`.",
                artifact.link
            )
        }

        links.insert(artifact.link.clone());
    }

    Ok(())
}

impl Validate for ManifestExposes {
    fn validate(&self) -> anyhow::Result<()> {
        validate_artifacts(&self.bin)
            .context("The exposed binaries were incorrectly configured.")?;
        validate_artifacts(&self.lib)
            .context("The exposed libraries were incorrectly configured.")?;

        for var in &self.env {
            var.validate()
                .context("The exposed environment variables were incorrectly configured.")?;
        }

        Ok(())
    }
}

impl ManifestExposes {
    fn is_empty(&self) -> bool {
        self.bin.is_empty() && self.lib.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ManifestExposesArtifact {
    pub link: String,
    pub path: PathBuf,
}

impl Validate for ManifestExposesArtifact {
    fn validate(&self) -> anyhow::Result<()> {
        validation::is_safe_path(&self.path).context(format!(
            "The specified artifact path `{}` is not safe.",
            self.path.display()
        ))?;

        if !(1..=32).contains(&self.link.len()) {
            anyhow::bail!(
                "Linked names must be at least one character long and up to 32 characters long."
            );
        }

        validation::is_safe_path_component(&self.link).context(format!(
            "The specified artifact linked name `{}` was not a safe path component.",
            self.link
        ))?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestExposesEnv {
    pub name: String,
    pub value: String,
}

impl Validate for ManifestExposesEnv {
    fn validate(&self) -> anyhow::Result<()> {
        let regex = Regex::new("^[a-zA-Z0-9_]{1,32}$")
            .context("This is a bug. The hard-coded validation regex was invalid.")?;

        if !regex.is_match(&self.name) {
            anyhow::bail!("The package contained an environment variable with an invalid name.");
        }

        if &self.name == "NYE_INSTALLATION" {
            anyhow::bail!(
                "NYE_INSTALLATION environment variable cannot be exposed, it's automatically set by nye."
            );
        }

        if self.value.len() > 512 {
            anyhow::bail!(
                "The package has an invalid variable exposed `{}`.",
                self.name
            );
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
    Value { name: String, value: String },
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
                "NYE_INSTALLATION environment variable cannot be consumed, it's always available."
            );
        }

        match &self {
            ManifestConsumesEnv::List { name: _, separator } => {
                if !(0..=16).contains(&separator.len()) {
                    anyhow::bail!(
                        "Consumed environment variables cannot have a separator longer than 16 bytes."
                    );
                }
            }
            ManifestConsumesEnv::Value { name: _, value } => {
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
