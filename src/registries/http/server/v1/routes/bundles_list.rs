use axum::extract::Query;
use serde::{Deserialize, Serialize};

use crate::registries::http::server::database::{Package, PackageVersionBundle};
use crate::registries::http::server::state::State;
use crate::registries::http::server::v1::errors::{Error, OptionOrHttpError, ResultOrHttpError};
use crate::registries::http::server::v1::schemas::Page;
use crate::semver::SemverQuery;
use crate::targets::Target;
use crate::validation::Validate;

#[derive(Serialize, Deserialize)]
pub struct BundleQueryParams {
    #[serde(default)]
    pub cursor: Option<u64>,

    #[serde(default, rename = "query")]
    pub queries: Vec<SemverQuery<u64>>,
    pub target: Target,
}

impl Validate for BundleQueryParams {
    fn validate(&self) -> anyhow::Result<()> {
        for query in &self.queries {
            if query.resource.is_none() {
                anyhow::bail!("All queries must have a resource.");
            }
        }

        if self.queries.len() > 100 {
            anyhow::bail!("You cannot make more than 100 queries per request.");
        }
        if self.queries.is_empty() {
            anyhow::bail!("You must specify at least one query with `?query=`.");
        }

        if !self.target.is_supported() {
            anyhow::bail!("You can only query package bundles for nye-supported targets.");
        }

        Ok(())
    }
}

pub async fn handle(state: State, params: Query<BundleQueryParams>) -> Result<Page<()>, Error> {
    params.validate().or_into_422()?;

    let mut db = state.db.connection().await.or_http_500()?;

    let mut expr = None;
    for query in &params.queries {
        let package_name = query.resource.as_ref().unwrap();
        expr = match expr {
            None => Some(Package::fields().name().eq(package_name)),
            Some(expr) => Some(expr.or(Package::fields().name().eq(package_name))),
        };
    }
    let query = Package::filter(expr.or_http_500()?)
        .include(Package::fields().versions())
        .include(
            Package::fields()
                .versions()
                .bundles()
                .filter(PackageVersionBundle::fields().target().eq(params.target.to_string())),
        )
        .exec(&mut db)
        .await
        .or_http_500()?;

    Ok(Page::new([], 0, 0))
}
