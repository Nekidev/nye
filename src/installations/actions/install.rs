use std::collections::{HashMap, HashSet};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context as AnyhowContext;
use askama::Template;
use async_zip::tokio::read::seek::ZipFileReader;
use jiff::Zoned;
use toasty::Transaction;
use tokio::fs::{self, File};
use tokio::io::{self, AsyncReadExt, BufReader};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::installations::context::Context;
use crate::installations::database::{self, ExposedArtifact, ExposedArtifactKind, Package};
use crate::installations::wrapper::{
    BinaryWrapper, BinaryWrapperBinary, BinaryWrapperDeclaredVariable, BinaryWrapperPackage,
};
use crate::packages::Manifest;
use crate::semver::Semver;
use crate::targets::Target;
use crate::validation::{self, Validate};

/// Installs an installable package file.
///
/// Arguments:
/// * `ctx` - The installation context.
/// * `path` - The path to the installable package file.
///
/// Returns:
/// * `Ok(Manifest)` - The installed package's manifest.
/// * `Err(Error)` - An error if any occurred while installing.
pub async fn install(ctx: Context, path: PathBuf) -> anyhow::Result<Manifest> {
    let mut zip = open_zip_file(&path)
        .await
        .context("Could not open installable package file. Is the file a package?")?;

    let manifest = get_manifest_from_zip(&mut zip)
        .await
        .context("Could not get manifest from package file. Is the file a package?")?;

    let current_target =
        Target::get_current().context("Could not get the current system's target.")?;
    if manifest.package.target != current_target {
        anyhow::bail!(
            "This package file was packed for a different target. Your system is `{}`, yet the package file was created for `{}`. Try with a file for your target instead.",
            current_target,
            manifest.package.target
        );
    }

    let paths = validate_zip_contents(&mut zip).context("The package file was invalid.")?;
    validate_manifest_exposed_bins(&manifest, &paths)
        .context("The package file's exposed bins were misconfigured.")?;
    validate_manifest_exposed_libs(&manifest, &paths)
        .context("The package file's exposed libs were misconfigured.")?;

    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;
    let mut transaction = db
        .transaction()
        .await
        .context("Could not start a transaction on the state database.")?;

    check_collissions(&mut transaction, &manifest)
        .await
        .context("An error occurred while checking for collissions.")?;

    extract_zip(&ctx, &mut zip, &manifest)
        .await
        .context("Failed to extract zip file.")?;

    expose_bins(&ctx, &manifest)
        .await
        .context("An error occurred while exposing binaries.")?;
    expose_libs(&ctx, &manifest)
        .await
        .context("An error occurred while exposing libraries.")?;
    expose_envs(&ctx, &manifest)
        .await
        .context("An error occurred while exposing environment variables.")?;

    update_state_database(&ctx, &manifest, &mut transaction)
        .await
        .context("Could not update state database after installing.")?;

    transaction
        .commit()
        .await
        .context("Could not commit state database transaction.")?;

    Ok(manifest)
}

async fn open_zip_file(path: &Path) -> anyhow::Result<ZipFileReader<BufReader<File>>> {
    let file = File::open(path)
        .await
        .context("Could not open package file for installation.")?;
    let zip = ZipFileReader::with_tokio(BufReader::new(file))
        .await
        .context("Could not read installable package file.")?;

    Ok(zip)
}

async fn get_manifest_from_zip(
    zip: &mut ZipFileReader<BufReader<File>>,
) -> anyhow::Result<Manifest> {
    if !zip.file().entries().is_empty() {
        for index in 0..zip.file().entries().len() {
            let entry = zip.file().entries().get(index).unwrap();
            let filename = entry
                .filename()
                .as_str()
                .context("Could not decode package file name.")?;

            if filename == "nye.toml" {
                if entry.uncompressed_size() > 1024 * 1024 {
                    anyhow::bail!("The manifest file in the package file was bigger than 1MB.");
                }

                let reader = zip.reader_without_entry(index).await.context(
                    "Could not get reader for manifest file in package file. This is weird.",
                )?;

                let mut string = String::new();
                reader
                    .compat()
                    .read_to_string(&mut string)
                    .await
                    .context("Could not read nye.toml manifest in package file to string. Are its contents correct?")?;

                let manifest: Manifest = toml::from_str(&string)
                    .context("The nye.toml manifest in the package file had invalid contents.")?;

                manifest
                    .validate()
                    .context("The package file's nye.toml manifest was invalid.")?;

                return Ok(manifest);
            }
        }
    }

    anyhow::bail!("The specified package file did not have any nye.toml manifest in it.");
}

fn validate_zip_contents(
    zip: &mut ZipFileReader<BufReader<File>>,
) -> anyhow::Result<HashSet<String>> {
    let mut paths = HashSet::new();

    for index in 0..zip.file().entries().len() {
        let entry = zip.file().entries().get(index).unwrap();
        let filename = entry.filename().clone().into_string().context(concat!(
            "A file in the package file could not have its name converted to a string. Is it ",
            "using weird characters?"
        ))?;

        validation::is_safe_path(&filename).context(format!(
            "The package file contained a file, `{filename}`, whose filename was not safe."
        ))?;

        let dir_prefixes = ["bin/", "lib/", "etc/", "var/"];

        if !dir_prefixes.iter().any(|i| filename.starts_with(i)) && filename != "nye.toml" {
            anyhow::bail!("The package file contained an out-of-place entry, `{filename}`.");
        }

        paths.insert(filename);
    }

    Ok(paths)
}

fn validate_manifest_exposed_bins(
    manifest: &Manifest,
    paths: &HashSet<String>,
) -> anyhow::Result<()> {
    for exposed_bin in &manifest.exposes.bin {
        let path = PathBuf::from("bin").join(&exposed_bin.path);

        if !paths.contains(&path.display().to_string()) {
            anyhow::bail!(
                "The package file specfied an exposed binary in its manifest that was not present, `{}`.",
                path.display()
            );
        }
    }

    Ok(())
}

fn validate_manifest_exposed_libs(
    manifest: &Manifest,
    paths: &HashSet<String>,
) -> anyhow::Result<()> {
    for exposed_lib in &manifest.exposes.lib {
        let path = PathBuf::from("lib").join(&exposed_lib.path);

        if !paths.contains(&path.display().to_string()) {
            anyhow::bail!(
                "The package file specfied an exposed library in its manifest that was not present, `{}`.",
                path.display()
            );
        }
    }

    Ok(())
}

async fn check_collissions(
    transaction: &mut Transaction<'_>,
    manifest: &Manifest,
) -> anyhow::Result<()> {
    check_package_version_collissions(transaction, manifest)
        .await
        .context("An error occurred while checking for package version collissions.")?;
    check_exposed_bin_collissions(transaction, manifest)
        .await
        .context("An error occurred while checking for exposed binary collissions.")?;
    check_exposed_lib_collissions(transaction, manifest)
        .await
        .context("An error occurred while checking for exposed library collissions.")?;

    Ok(())
}

async fn check_package_version_collissions(
    transaction: &mut Transaction<'_>,
    manifest: &Manifest,
) -> anyhow::Result<()> {
    let packages = Package::all()
        .exec(transaction)
        .await
        .context("Could not get all installed packages from state database.")?;

    for package in &packages {
        if package.name != manifest.package.name {
            continue;
        }

        let version: Semver<u64> = Semver::from_str(&package.version).context(format!(
            "Package `{} v{}` stored in state database did not have a valid semver version.",
            package.name, package.version
        ))?;

        if version == manifest.package.version {
            anyhow::bail!(
                "Package `{} v{}` is already installed.",
                package.name,
                package.version
            );
        }
    }

    Ok(())
}

async fn check_exposed_bin_collissions(
    transaction: &mut Transaction<'_>,
    manifest: &Manifest,
) -> anyhow::Result<()> {
    let binaries = ExposedArtifact::all()
        .filter_by_kind(ExposedArtifactKind::Binary)
        .exec(transaction)
        .await
        .context("Could not get all exposed binaries from state database.")?;

    for binary_in_state in &binaries {
        for binary_in_manifest in &manifest.exposes.bin {
            if binary_in_state.name == binary_in_manifest.link {
                anyhow::bail!(
                    "The exposed binary `{}` in this package collides with an already-installed binary.",
                    binary_in_manifest.link
                );
            }
        }
    }

    Ok(())
}

async fn check_exposed_lib_collissions(
    transaction: &mut Transaction<'_>,
    manifest: &Manifest,
) -> anyhow::Result<()> {
    let libraries = ExposedArtifact::all()
        .filter_by_kind(ExposedArtifactKind::Library)
        .exec(transaction)
        .await
        .context("Could not get all exposed libraries from state database.")?;

    for library_in_state in &libraries {
        for library_in_manifest in &manifest.exposes.lib {
            if library_in_state.name == library_in_manifest.link {
                anyhow::bail!(
                    "The exposed library `{}` in this package collides with an already-installed library.",
                    library_in_manifest.link
                );
            }
        }
    }

    Ok(())
}

async fn extract_zip(
    ctx: &Context,
    zip: &mut ZipFileReader<BufReader<File>>,
    manifest: &Manifest,
) -> anyhow::Result<()> {
    let output_dir = ctx
        .root
        .join("pkg")
        .join("store")
        .join(&manifest.package.name)
        .join(manifest.package.version.to_string());

    fs::create_dir_all(&output_dir)
        .await
        .context("Could not create output directory.")?;

    for index in 0..zip.file().entries().len() {
        let file = zip.file().entries().get(index).unwrap();

        if file
            .dir()
            .context("Could not check whether package file was directory.")?
        {
            continue;
        }

        let filename = file
            .filename()
            .clone()
            .into_string()
            .context("Could not read filename of file in package file.")?;

        let output_filename = output_dir.join(&filename);
        fs::create_dir_all(
            output_filename
                .parent()
                .context("Absolute file path for file in package file had no parent.")?,
        )
        .await
        .context(
            "Could not ensure all required parent directories existed for file in package file.",
        )?;

        let mut output_file = File::create_new(&output_filename).await.context(format!(
            "Could not create file `{}` in system to extract file from package file.",
            output_filename.display()
        ))?;

        let entry = zip
            .reader_without_entry(index)
            .await
            .context("Could not get reader for file in package file.")?;

        io::copy(&mut entry.compat(), &mut output_file)
            .await
            .context("Could not copy data from file in package file to system file.")?;

        let permissions = {
            if ctx.is_system {
                if filename.starts_with("bin/") {
                    Permissions::from_mode(0o755)
                } else {
                    Permissions::from_mode(0o644)
                }
            } else {
                if filename.starts_with("bin/") {
                    Permissions::from_mode(0o700)
                } else {
                    Permissions::from_mode(0o600)
                }
            }
        };

        output_file
            .set_permissions(permissions)
            .await
            .context("Could not update permissions of file.")?;
    }

    Ok(())
}

async fn expose_bins(ctx: &Context, manifest: &Manifest) -> anyhow::Result<()> {
    for bin in &manifest.exposes.bin {
        let installation = ctx
            .root
            .join("pkg")
            .join("store")
            .join(&manifest.package.name)
            .join(manifest.package.version.to_string());
        let original = installation.join("bin").join(&bin.path);
        let link = ctx.root.join("bin").join(&bin.link);

        let wrapper = BinaryWrapper {
            namespace: ctx.root.display().to_string(),
            binary: BinaryWrapperBinary {
                path: original.display().to_string(),
            },
            package: BinaryWrapperPackage {
                name: manifest.package.name.clone(),
                version: manifest.package.version.clone(),
            },
            consumed_variables: vec![],
            declared_variables: vec![BinaryWrapperDeclaredVariable {
                name: String::from("NYE_INSTALLATION"),
                value: installation.display().to_string(),
            }],
        };

        let mut script = wrapper
            .render()
            .context("Could not render binary wrapper shell script.")?;

        loop {
            if script.replace("\n\n\n", "\n\n") != script {
                script = script.replace("\n\n\n", "\n\n");
            } else {
                break;
            }
        }

        fs::write(&link, script.as_bytes())
            .await
            .context("Could not write binary wrapper shell script to link location.")?;

        let permissions = if ctx.is_system {
            Permissions::from_mode(0o555)
        } else {
            Permissions::from_mode(0o544)
        };

        fs::set_permissions(&link, permissions)
            .await
            .context("Could not set execution permissions to link.")?;
    }

    Ok(())
}

async fn expose_libs(ctx: &Context, manifest: &Manifest) -> anyhow::Result<()> {
    for lib in &manifest.exposes.lib {
        let original = ctx
            .root
            .join("pkg")
            .join("store")
            .join(&manifest.package.name)
            .join(manifest.package.version.to_string())
            .join("lib")
            .join(&lib.path);
        let link = ctx.root.join("lib").join(&lib.link);

        fs::symlink(original, link)
            .await
            .context("Could not expose library from package file.")?;
    }

    Ok(())
}

async fn expose_envs(ctx: &Context, manifest: &Manifest) -> anyhow::Result<()> {
    let mut counters = HashMap::new();

    for var in &manifest.exposes.env {
        let counter = counters
            .entry(var.name.clone())
            .and_modify(|v| *v += 1)
            .or_insert(0);

        let location_dir = ctx
            .root
            .join("env")
            .join(&var.name)
            .join(&manifest.package.name)
            .join(&manifest.package.version.to_string());
        let location_file = location_dir.join(format!("{counter}.txt"));

        fs::create_dir_all(location_dir)
            .await
            .context("Could not create required directories to store env var file.")?;

        let value = shellexpand::env_with_context_no_errors(&var.value, |var_name| {
            if var_name == "NYE_INSTALLATION" {
                Some(
                    ctx.get_package_installation_path(
                        &manifest.package.name,
                        &manifest.package.version,
                    )
                    .display()
                    .to_string(),
                )
            } else {
                None
            }
        }).to_string();

        fs::write(&location_file, &value)
            .await
            .context("Could not write environment variable value to file.")?;
        fs::set_permissions(&location_file, Permissions::from_mode(0o444))
            .await
            .context("Could not set read-only permissions to env var file.")?;
    }

    Ok(())
}

async fn update_state_database(
    ctx: &Context,
    manifest: &Manifest,
    transaction: &mut Transaction<'_>,
) -> anyhow::Result<()> {
    let package_installation_path =
        ctx.get_package_installation_path(&manifest.package.name, &manifest.package.version);

    toasty::create!(Package {
        name: manifest.package.name.clone(),
        version: manifest.package.version.to_string(),
        location: package_installation_path.display().to_string(),
        created_at: Zoned::now(),
        updated_at: Zoned::now(),
    })
    .exec(transaction)
    .await
    .context("Could not insert package into state database.")?;

    for bin in &manifest.exposes.bin {
        toasty::create!(ExposedArtifact {
            name: bin.link.clone(),
            kind: ExposedArtifactKind::Binary,
            location: package_installation_path
                .join("bin")
                .join(&bin.path)
                .display()
                .to_string(),
            package_name: manifest.package.name.clone(),
            created_at: Zoned::now(),
            updated_at: Zoned::now(),
        })
        .exec(transaction)
        .await
        .context("Could not insert exposed bin into state database.")?;
    }

    for lib in &manifest.exposes.lib {
        toasty::create!(ExposedArtifact {
            name: lib.link.clone(),
            kind: ExposedArtifactKind::Library,
            location: package_installation_path
                .join("lib")
                .join(&lib.path)
                .display()
                .to_string(),
            package_name: manifest.package.name.clone(),
            created_at: Zoned::now(),
            updated_at: Zoned::now(),
        })
        .exec(transaction)
        .await
        .context("Could not insert exposed lib into state database.")?;
    }

    for var in &manifest.exposes.env {
        toasty::create!(ExposedArtifact {
            name: var.name.clone(),
            kind: ExposedArtifactKind::Variable,
            location: ctx
                .root
                .join("env")
                .join(&var.name)
                .join(&manifest.package.name)
                .join(&manifest.package.version.to_string())
                .display()
                .to_string(),
            package_name: manifest.package.name.clone(),
            created_at: Zoned::now(),
            updated_at: Zoned::now(),
        })
        .exec(transaction)
        .await
        .context("Could not insert exposed environment variable into state database.")?;
    }

    Ok(())
}
