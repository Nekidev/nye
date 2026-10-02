//! Installation state, containing metadata about installed packages.
//!
//! <div class="warning">
//! It is not recommended to touch the database using this module, as doing so incorrectly will
//! break the namespace.
//! </div>

use std::fmt::Display;

use anyhow::Context;
use toasty::migration::MigrationSet;
use toasty::{Db, Deferred, Embed, Model};

#[derive(Model)]
pub struct Package {
    #[key]
    pub name: String,

    /// The full path to where the package's versions are installed.
    ///
    /// E.g. `/usr/root/pkg/store/busybox`
    ///
    /// Note: It may or may not end with a trailing slash. It will always be absolute.
    pub path: String,

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

    /// The full path to where this version's files are installed.
    ///
    /// E.g. `/usr/root/pkg/store/busybox/1.0.0`
    ///
    /// Note: It may or may not end with a trailing slash. It will always be absolute.
    pub path: String,

    #[has_many]
    pub artifacts: Deferred<Vec<Artifact>>,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Model)]
#[index(package_name, version_number)]
#[index(kind, link)]
pub struct Artifact {
    #[key]
    #[auto]
    pub id: u64,

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
    pub path: String,
    /// The artifact kind.
    pub kind: ArtifactKind,
    /// The names under which the artifact is exposed.
    ///
    /// This is what this value stores when the artifact is of each kind:
    /// * `bin` - The name under which the artifact is exposed.
    /// * `lib` - The name under which the artifact is exposed.
    /// * `env` - The name of the environment variable.
    pub link: String,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Embed, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Bin,
    Lib,
    Env,
    Var,
}

impl Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bin => write!(f, "bin"),
            Self::Lib => write!(f, "lib"),
            Self::Env => write!(f, "env"),
            Self::Var => write!(f, "var"),
        }
    }
}

const MIGRATIONS: MigrationSet = toasty::embed_migrations!();

pub async fn connect(url: impl Into<String>) -> anyhow::Result<Db> {
    let db = Db::builder()
        .models(toasty::models!(Package, Version, Artifact))
        .connect(url.into().as_str())
        .await
        .context("Could not connect to namespace database.")?;

    MIGRATIONS
        .apply(&db)
        .await
        .context("Could not apply migrations to namespace database.")?;

    Ok(db)
}
