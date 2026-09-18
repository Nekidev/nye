use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Context;
use async_zip::tokio::read::seek::ZipFileReader;
use nye_validation::Validate;
use tokio::io::{AsyncBufRead, AsyncSeek};

use crate::extraction;
use crate::manifest::Manifest;

pub fn validate<T>(zip: &mut ZipFileReader<T>, manifest: &Manifest) -> anyhow::Result<()>
where
    T: AsyncBufRead + AsyncSeek + Unpin,
{
    manifest.validate().context("The package's manifest was invalid.")?;

    if zip.file().entries().len() > 512 {
        anyhow::bail!("Package files must not contain more than 512 files.");
    }

    let paths = extraction::read_paths(zip).context("Could not read paths in package file.")?;

    validate_files(&paths).context("The files in the package file were invalid or unsafe.")?;
    validate_exposed_bins(&paths, manifest).context("The manifest exposed invalid binaries.")?;
    validate_exposed_libs(&paths, manifest).context("The manifest exposed invalid libraries.")?;

    Ok(())
}

fn validate_files(paths: &HashSet<PathBuf>) -> anyhow::Result<()> {
    'paths: for path in paths {
        nye_validation::is_safe_path(path)
            .context("A file path in the package file was not safe.")?;

        let prefixes = ["bin/", "lib/", "var/", "etc/"];

        if path == "nye.toml" {
            continue;
        }

        for prefix in prefixes {
            if path.starts_with(prefix) {
                continue 'paths;
            }
        }

        anyhow::bail!("The path `{}` in the package file is out of place.", path.display());
    }

    Ok(())
}

fn validate_exposed_bins(paths: &HashSet<PathBuf>, manifest: &Manifest) -> anyhow::Result<()> {
    for bin in &manifest.exposes.bin {
        if !paths.contains(&bin.path) {
            anyhow::bail!("The package exposed a binary that was not in the package file.");
        }
    }

    Ok(())
}

fn validate_exposed_libs(paths: &HashSet<PathBuf>, manifest: &Manifest) -> anyhow::Result<()> {
    for lib in &manifest.exposes.lib {
        if !paths.contains(&lib.path) {
            anyhow::bail!("The package exposed a library that was not in the package file.");
        }
    }

    Ok(())
}
