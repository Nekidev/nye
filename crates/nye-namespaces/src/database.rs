//! Installation state, containing metadata about installed packages.

use toasty::{Deferred, Embed, Model};

#[derive(Model)]
pub struct Package {
    #[key]
    pub name: String,

    #[has_many]
    pub versions: Deferred<Vec<Version>>,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Model)]
#[key(package_name, number)]
pub struct Version {
    pub number: String,

    #[belongs_to(key = package_name, references = name)]
    pub package: Deferred<Package>,
    pub package_name: String,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Model)]
#[key(package_name, version_number, kind, path)]
pub struct Artifact {
    #[belongs_to(key = package_name, references = name)]
    pub package: Deferred<Package>,
    pub package_name: String,

    #[belongs_to(key = [package_name, version_number], references = [package_name, number])]
    pub version: Deferred<Version>,
    pub version_number: String,

    /// The full path within the installation's store to the artifact file.
    /// 
    /// This is what this value stores when the artifact is of each kind:
    /// * `bin` - The path to the binary.
    /// * `lib` - The path to the library.
    /// * `env` - The path to the environment variable value file.
    #[unique]
    pub path: String,
    /// The artifact kind.
    pub kind: ArtifactKind,
    /// The names under which the artifact is exposed.
    /// 
    /// This is what this value stores when the artifact is of each kind:
    /// * `bin` - Names under which the artifact is exposed.
    /// * `lib` - Names under which the artifact is exposed.
    /// * `env` - The name of the environment variable.
    pub links: Vec<String>,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Embed)]
pub enum ArtifactKind {
    Bin,
    Lib,
    Env,
}
