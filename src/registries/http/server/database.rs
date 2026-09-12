use anyhow::Context;
use toasty::{Db, Deferred};

static MIGRATIONS: toasty::migration::MigrationSet =
    toasty::embed_migrations!("src/registries/http/server/toasty");

pub async fn connect(url: impl Into<String>) -> anyhow::Result<Db> {
    let url = url.into();

    let db = toasty::Db::builder()
        .models(toasty::models!(User, Package, PackageVersion, PackageVersionBundle))
        .connect(&url)
        .await
        .context("Could not connect to the database.")?;

    MIGRATIONS
        .apply(&db)
        .await
        .context(format!("Could not apply migrations to `{url}`."))?;

    Ok(db)
}

#[derive(toasty::Model, Clone)]
pub struct User {
    #[key]
    pub id: String,

    #[unique]
    pub name: String,
    #[unique]
    pub email: String,
    pub password: String,

    #[has_many(pair = user)]
    pub tokens: Deferred<Vec<Token>>,
    #[has_many(pair = user)]
    pub packages: Deferred<Vec<Package>>,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(toasty::Model, Clone)]
pub struct Token {
    #[key]
    pub id: String,

    pub kind: TokenKind,

    #[belongs_to]
    pub user: Deferred<User>,
    #[index]
    pub user_id: String,

    pub created_at: u64,
    pub updated_at: u64,
    pub expires_at: u64,
}

#[derive(toasty::Embed, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(toasty::Model, Clone)]
pub struct Package {
    #[key]
    pub id: String,
    #[unique]
    pub name: String,

    #[belongs_to]
    pub user: Deferred<User>,
    #[index]
    pub user_id: String,

    #[has_many(pair = package)]
    pub versions: Deferred<Vec<PackageVersion>>,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(toasty::Model, Clone)]
#[unique(package_id, number)]
pub struct PackageVersion {
    #[key]
    pub id: String,
    pub number: String,

    #[belongs_to]
    pub package: Deferred<Package>,
    #[index]
    pub package_id: String,

    #[has_many(pair = version)]
    pub bundles: Deferred<Vec<PackageVersionBundle>>,

    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(toasty::Model, Clone)]
#[unique(version_id, target)]
pub struct PackageVersionBundle {
    #[key]
    pub id: String,
    pub target: String,

    pub file: String,

    #[belongs_to]
    pub version: Deferred<PackageVersion>,
    #[index]
    pub version_id: String,

    pub created_at: u64,
    pub updated_at: u64,
}
