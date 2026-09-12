use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
use async_zip::tokio::read::seek::ZipFileReader;
use tokio::io::{AsyncBufRead, AsyncReadExt, AsyncSeek};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::packages::Manifest;
use crate::validation::{self, Validate};

pub async fn validate<T>(zip: &mut ZipFileReader<T>) -> anyhow::Result<Manifest>
where
    T: AsyncBufRead + AsyncSeek + Unpin,
{
    check_zip_safety(zip).context("The package zip file was unsafe.")?;

    let manifest = get_manifest_from_zip(zip)
        .await
        .context("Could not get nye.toml manifest in zip file.")?;

    let paths = check_zip_contents(zip).context("Could not check zip contents.")?;
    check_manifest_exposed_bins(&manifest, &paths)
        .context("The exposed binaries of the package were misconfigured.")?;
    check_manifest_exposed_libs(&manifest, &paths)
        .context("The exposed libraries of the package were misconfigured.")?;

    Ok(manifest)
}

fn check_zip_safety<T>(zip: &mut ZipFileReader<T>) -> anyhow::Result<()>
where
    T: AsyncBufRead + AsyncSeek + Unpin,
{
    if zip.file().entries().len() > 512 {
        anyhow::bail!("The package zip file contains more than 512 files in it.");
    }

    Ok(())
}

fn check_zip_contents<T>(zip: &mut ZipFileReader<T>) -> anyhow::Result<HashSet<String>>
where
    T: AsyncBufRead + AsyncSeek + Unpin,
{
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

fn check_manifest_exposed_bins(manifest: &Manifest, paths: &HashSet<String>) -> anyhow::Result<()> {
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

fn check_manifest_exposed_libs(manifest: &Manifest, paths: &HashSet<String>) -> anyhow::Result<()> {
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

pub async fn get_manifest_from_zip<T>(zip: &mut ZipFileReader<T>) -> anyhow::Result<Manifest>
where
    T: AsyncBufRead + AsyncSeek + Unpin,
{
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
