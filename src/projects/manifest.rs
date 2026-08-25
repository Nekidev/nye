use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use serde::de::Visitor;
use serde::{Deserialize, Serialize};

use crate::semver::Semver;
use crate::targets::Target;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetOrShared {
    Shared,
    Target(Target),
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

                let target = Target::from_str(v).map_err(|e| {
                    E::custom("The specified target was neither a valid target nor `shared`.")
                })?;

                Ok(TargetOrShared::Target(target))
            }
        }

        deserializer.deserialize_str(TargetOrSharedVisitor)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub package: ManifestPackage,
    pub targets: HashMap<TargetOrShared, ManifestTarget>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestPackage {
    pub name: String,
    pub version: Semver,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestTarget {
    pub source: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestExposes {
    pub bin: Vec<ManifestExposesBin>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestExposesBin {
    pub path: PathBuf,

    /// When empty, it defaults to the file name.
    #[serde(default)]
    pub links: Vec<String>,

    /// When empty, it defaults to all targets supported by the package.
    #[serde(default)]
    pub targets: Vec<Target>,
}
