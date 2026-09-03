use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
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
        let mut bin = Vec::new();
        for exposed_bin in manifest.exposes.bin {
            if (!exposed_bin.targets.is_empty() && exposed_bin.targets.contains(&target))
                || exposed_bin.targets.is_empty()
            {
                for link in exposed_bin.links {
                    bin.push(ManifestExposesBin {
                        link,
                        path: exposed_bin.path.clone(),
                    });
                }
            }
        }

        Manifest {
            package: ManifestPackage {
                name: manifest.package.name,
                version: manifest.package.version,
                target,
            },
            exposes: ManifestExposes { bin },
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
    pub bin: Vec<ManifestExposesBin>,
}

impl Validate for ManifestExposes {
    fn validate(&self) -> anyhow::Result<()> {
        let mut links = HashSet::new();

        for bin in &self.bin {
            bin.validate().context(format!(
                "The exposed binary `{}` was incorrectly configured.",
                bin.path.display()
            ))?;

            if links.contains(&bin.link) {
                anyhow::bail!(
                    "Two or more exposed binaries conflict on the linked name `{}`.",
                    bin.link
                )
            }

            links.insert(bin.link.clone());
        }

        Ok(())
    }
}

impl ManifestExposes {
    fn is_empty(&self) -> bool {
        self.bin.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ManifestExposesBin {
    pub link: String,
    pub path: PathBuf,
}

impl Validate for ManifestExposesBin {
    fn validate(&self) -> anyhow::Result<()> {
        validation::is_safe_path(&self.path).context(format!(
            "The specified path `{}` is not safe.",
            self.path.display()
        ))?;

        if !(1..=32).contains(&self.link.len()) {
            anyhow::bail!(
                "Linked names must be at least one character long and up to 32 characters long."
            );
        }

        validation::is_safe_path_component(&self.link).context(format!(
            "The specified linked name `{}` was not a safe path component.",
            self.link
        ))?;

        Ok(())
    }
}
