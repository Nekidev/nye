use anyhow::Context;
use jiff::Zoned;
use toasty::{Db, Deferred};

static MIGRATIONS: toasty::migration::MigrationSet =
    toasty::embed_migrations!("src/installations/toasty");

#[derive(toasty::Model)]
pub struct Package {
    #[key]
    pub name: String,
    pub version: String,

    #[has_many]
    pub exposes_bins: Deferred<Vec<ExposedBin>>,
    #[has_many]
    pub exposes_libs: Deferred<Vec<ExposedLib>>,

    pub created_at: Zoned,
    pub updated_at: Zoned,
}

#[derive(toasty::Model)]
pub struct ExposedBin {
    #[key]
    pub name: String,

    #[belongs_to(key = package_name, references = name)]
    pub package: Deferred<Package>,
    #[index]
    pub package_name: String,

    pub created_at: Zoned,
    pub updated_at: Zoned,
}

#[derive(toasty::Model)]
pub struct ExposedLib {
    #[key]
    pub name: String,

    #[belongs_to(key = package_name, references = name)]
    pub package: Deferred<Package>,
    #[index]
    pub package_name: String,

    pub created_at: Zoned,
    pub updated_at: Zoned,
}

pub async fn connect(url: impl Into<String>) -> anyhow::Result<Db> {
    let url = url.into();
    
    let db = toasty::Db::builder()
        .models(toasty::models!(Package, ExposedBin, ExposedLib))
        .connect(&url)
        .await
        .context("Could not connect to the database.")?;

    MIGRATIONS
        .apply(&db)
        .await
        .context(format!("Could not apply migrations to `{url}`."))?;

    Ok(db)
}
