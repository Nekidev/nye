use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
use nye_schemas::semver::Semver;
use nye_schemas::targets::Target;
use nye_validation::Validate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: ManifestPackage,

    #[serde(default, skip_serializing_if = "ManifestExposes::is_empty")]
    pub exposes: ManifestExposes,

    #[serde(default, skip_serializing_if = "ManifestConsumes::is_empty")]
    pub consumes: ManifestConsumes,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ManifestPackage {
    /// The package's name.
    pub name: String,
    /// The package's version.
    pub version: Semver,
    /// The target this package file was made for.
    /// 
    /// This value may contain an unsupported target. Make sure to validate it using
    /// [`Target::is_supported()`] before processing.
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

        nye_validation::is_kebab_case(&self.name)
            .context("The package name was not valid kebab case")?;

        if !self.target.is_supported() {
            anyhow::bail!("The configured target in the manifest is not supported by nye.");
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
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
        self.bin.is_empty() && self.lib.is_empty() && self.env.is_empty()
    }
}

#[derive(Clone, Serialize, Deserialize)]
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

impl Validate for ManifestExposesArtifact {
    fn validate(&self) -> anyhow::Result<()> {
        nye_validation::is_safe_path(&self.path).context(format!(
            "The specified artifact path `{}` is not safe.",
            self.path.display()
        ))?;

        if !(1..=32).contains(&self.link.len()) {
            anyhow::bail!(
                "Linked names must be at least one character long and up to 32 characters long."
            );
        }

        nye_validation::is_safe_path_component(&self.link).context(format!(
            "The specified artifact linked name `{}` was not a safe path component.",
            self.link
        ))?;

        Ok(())
    }
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

impl Validate for ManifestExposesEnv {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=32).contains(&self.name.len()) {
            anyhow::bail!(
                "Environment variable names must have at least 1 character and no more than 32."
            );
        }

        nye_validation::is_env_var_name(&self.name)
            .context("The manifest exposes an invalid environment variable name.")?;

        if &self.name == "NYE_INSTALLATION" {
            anyhow::bail!(
                "NYE_INSTALLATION environment variable cannot be exposed, it's automatically set by nye."
            );
        }

        if self.value.len() > 512 {
            anyhow::bail!(
                "Environment variable exposed values must not exceed 512 characters. `{}` did.",
                self.name
            );
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
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

impl Validate for ManifestConsumesEnv {
    fn validate(&self) -> anyhow::Result<()> {
        nye_validation::is_env_var_name(self.name())
            .context("The manifest consumes an invalid environment variable name.")?;

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
