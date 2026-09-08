use anyhow::Context;
use toasty::Db;

static MIGRATIONS: toasty::migration::MigrationSet =
    toasty::embed_migrations!("src/registries/http/server/toasty");

pub async fn connect(url: impl Into<String>) -> anyhow::Result<Db> {
    let url = url.into();

    let db = toasty::Db::builder()
        .models(toasty::models!())
        .connect(&url)
        .await
        .context("Could not connect to the database.")?;

    MIGRATIONS
        .apply(&db)
        .await
        .context(format!("Could not apply migrations to `{url}`."))?;

    Ok(db)
}
