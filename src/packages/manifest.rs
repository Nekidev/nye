use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::projects;
use crate::semver::Semver;
use crate::targets::Target;

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

#[derive(Serialize, Deserialize)]
pub struct ManifestPackage {
    pub name: String,
    pub version: Semver,
    pub target: Target,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ManifestExposes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<ManifestExposesBin>,
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
