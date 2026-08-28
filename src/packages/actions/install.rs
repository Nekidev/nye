use std::path::{Path, PathBuf};

use anyhow::Context as AnyhowContext;
use async_zip::tokio::read::seek::ZipFileReader;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::packages::Manifest;
use crate::packages::context::Context;
use crate::validation;

/// Installs an installable package file.
///
/// Arguments:
/// * `ctx` - The installation context.
/// * `path` - The path to the installable package file.
pub async fn install(ctx: Context, path: PathBuf) -> anyhow::Result<()> {
    let mut zip = open_zip_file(&path)
        .await
        .context("Could not open installable package file. Is the file a package?")?;

    let manifest = get_manifest_from_zip(&mut zip)
        .await
        .context("Could not get manifest from package file. Is the file a package?")?;

    validate_zip(&manifest, &mut zip).context("The package file was not valid.")?;

    todo!()
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

                return Ok(manifest);
            }
        }
    }

    anyhow::bail!("The specified package file did not have any nye.toml manifest in it.");
}

fn validate_zip(
    manifest: &Manifest,
    zip: &mut ZipFileReader<BufReader<File>>,
) -> anyhow::Result<()> {
    for index in 0..zip.file().entries().len() {
        let entry = zip.file().entries().get(index).unwrap();
        let filename = entry.filename().as_str().context(concat!(
            "A file in the package file could not have its name converted to a string. Is it ",
            "using weird characters?"
        ))?;

        validation::is_safe_path(filename).context(format!(
            "The package file contained a file, `{filename}`, whose filename was not safe."
        ))?;

        let dir_prefixes = ["bin/", "lib/", "etc/", "var/"];

        if !dir_prefixes.iter().any(|i| filename.starts_with(i)) && filename != "nye.toml" {
            anyhow::bail!("The package file contained an out-of-place entry, `{filename}`.");
        }
    }

    todo!("validate manifest, implement Validate on it and make sure what's exposed makes sense.");

    Ok(())
}
