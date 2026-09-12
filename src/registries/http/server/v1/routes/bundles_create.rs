use std::collections::{HashMap, HashSet};

use async_zip::tokio::read::seek::ZipFileReader;
use axum::extract::Multipart;
use tokio::fs::{self, File};
use tokio::io::{AsyncWriteExt, BufReader};

use crate::deferred::DeferredGroup;
use crate::packages::extraction;
use crate::registries::http::server::database::{Package, PackageVersion, PackageVersionBundle};
use crate::registries::http::server::state::State;
use crate::registries::http::server::storage::Storage;
use crate::registries::http::server::v1::errors::{Error, ResultOrHttpError};
use crate::registries::http::server::v1::extractors::auth::Auth;
use crate::semver::Semver;
use crate::targets::Target;
use crate::time;

struct PackageMeta {
    name: String,
    version: Semver,
    targets: HashSet<Target>,
}

pub async fn handle(state: State, auth: Auth, mut multipart: Multipart) -> Result<(), Error> {
    let mut db = state.db.connection().await.or_http_500()?;
    let mut db = db.transaction().await.or_http_500()?;
    let mut package_meta: Option<PackageMeta> = None;
    let mut package_bundles: HashMap<Target, String> = HashMap::new();

    let mut deferred = DeferredGroup::default();

    while let Some(mut field) = multipart.next_field().await.or_into_422()? {
        let filename = format!(".temp_upload_{}", nanoid::nanoid!());
        let mut file = File::create_new(&filename).await.or_http_500()?;
        deferred.defer(fs::remove_file(filename.clone()));

        let mut size = 0;
        while let Some(chunk) = field.chunk().await.or_http_422()? {
            size += chunk.len();

            if size >= 10 * 1024 * 1024 {
                return Err(Error::http_413());
            }

            file.write_all(&chunk).await.or_http_500()?;
        }

        let mut zip = ZipFileReader::with_tokio(BufReader::new(file))
            .await
            .or_http_422()?;
        let manifest = extraction::validate(&mut zip).await.or_http_422()?;

        if !manifest.package.target.is_supported() {
            return Err(Error::http_422());
        }

        if let Some(package_meta) = &mut package_meta {
            if package_meta.name != manifest.package.name {
                return Err(Error::http_422());
            }

            if package_meta.version != manifest.package.version {
                return Err(Error::http_422());
            }

            if package_meta.targets.contains(&manifest.package.target) {
                return Err(Error::http_422());
            }

            package_meta.targets.insert(manifest.package.target);
        } else {
            let package = Package::filter_by_name(&manifest.package.name)
                .first()
                .exec(&mut db)
                .await
                .or_http_500()?;

            if let Some(package) = package {
                if package.user_id != auth.user.id {
                    return Err(Error::new_403(
                        "Not Your Package",
                        "You don't own this package, so you cannot upload bundles for it. If this is your first time uploading this package, the name is already taken; pick a new one.",
                    ));
                }

                let version = PackageVersion::filter_by_package_id_and_number(
                    &package.id,
                    manifest.package.version.to_string(),
                )
                .first()
                .exec(&mut db)
                .await
                .or_http_500()?;

                if version.is_some() {
                    return Err(Error::new_409(
                        "This Version Already Exists",
                        "This version already exists for this package. Bump your package's version before re-uploading.",
                    ));
                }
            } else {
                toasty::create!(Package {
                    id: nanoid::nanoid!(),
                    name: &manifest.package.name,
                    user_id: &auth.user.id,
                    created_at: time::utc_now_ms(),
                    updated_at: time::utc_now_ms(),
                })
                .exec(&mut db)
                .await
                .or_http_500()?;
            }

            package_meta = Some(PackageMeta {
                name: manifest.package.name.clone(),
                version: manifest.package.version.clone(),
                targets: HashSet::from([manifest.package.target]),
            });
        }

        package_bundles.insert(manifest.package.target, filename);
    }

    let Some(package_meta) = package_meta else {
        return Err(Error::http_422());
    };

    let version = toasty::create!(PackageVersion {
        id: nanoid::nanoid!(),
        number: package_meta.version.to_string(),
        created_at: time::utc_now_ms(),
        updated_at: time::utc_now_ms(),
    })
    .exec(&mut db)
    .await
    .or_http_500()?;

    for (target, filename) in package_bundles {
        let key = format!(
            "packages/{}/versions/{}/bundles/{}.zip",
            package_meta.name, package_meta.version, target
        );

        state
            .storage
            .set_object(&key, filename)
            .await
            .or_http_500()?;

        toasty::create!(PackageVersionBundle {
            id: nanoid::nanoid!(),
            target: target.to_string(),
            file: key,
            version_id: &version.id,
            created_at: time::utc_now_ms(),
            updated_at: time::utc_now_ms(),
        })
        .exec(&mut db)
        .await
        .or_http_500()?;
    }

    db.commit().await.or_http_500()?;

    Ok(())
}
