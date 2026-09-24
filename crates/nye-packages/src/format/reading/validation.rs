//! Validate a package file's manifest against the file's contents.

use std::str::FromStr;

use anyhow::Context;
use nye_schemas::targets::Target;

use crate::format::safety::Safety;
use crate::format::{NyeFileDirectory, NyeFileEntryKind, Segments};
use crate::manifest::{Manifest, ManifestExposesArtifact, ManifestExposesEnv};

pub fn validate(
    manifest: &Manifest,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    validate_package_meta(manifest, safety)
        .context("The package's metadata was incorrectly configured.")?;
    validate_package_exposes(manifest, directory, safety)
        .context("The package's exposed artifacts were incorrectly configured.")?;

    Ok(())
}

fn validate_package_meta(manifest: &Manifest, safety: &Safety) -> anyhow::Result<()> {
    if manifest.package.name.len() as u64 > safety.max_package_name_size {
        anyhow::bail!("The package's name was longer than {} bytes.", safety.max_package_name_size);
    }

    if manifest.package.name.is_empty() {
        anyhow::bail!("The package had no name (empty string).");
    }

    nye_validation::is_kebab_case(&manifest.package.name)
        .context("The package's name was not kebab case.")?;

    if manifest.package.version.to_string().len() as u64 > safety.max_package_version_size {
        anyhow::bail!("The package's version string was longer than the max allowed.");
    }

    if !manifest.package.target.is_supported() && safety.only_supported_targets {
        anyhow::bail!(
            "The package's target, {}, is not supported by nye.",
            manifest.package.target
        );
    }

    let current_target = Target::get_current()?;
    if safety.only_current_target && current_target != manifest.package.target {
        anyhow::bail!(
            "Only the current system's target, {}, is allowed, but this package was made for {}.",
            current_target,
            manifest.package.target
        );
    }

    Ok(())
}

fn validate_package_exposes(
    manifest: &Manifest,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    // TODO: Validate artifact name uniqueness per artifact type.

    if manifest.exposes.bin.len() as u64 > safety.max_exposed_bins {
        anyhow::bail!("The manifest exposed more binaries than allowed.");
    }
    if manifest.exposes.lib.len() as u64 > safety.max_exposed_libs {
        anyhow::bail!("The manifest exposed more libraries than allowed.");
    }
    if manifest.exposes.env.len() as u64 > safety.max_exposed_vars {
        anyhow::bail!("The manifest exposed more environment variables than allowed.");
    }

    for bin in &manifest.exposes.bin {
        validate_exposed_artifact(bin, NyeFileEntryKind::Bin, directory, safety)
            .context("An exposed binary was misconfigured.")?;
    }
    for lib in &manifest.exposes.lib {
        validate_exposed_artifact(lib, NyeFileEntryKind::Lib, directory, safety)
            .context("An exposed library was misconfigured.")?;
    }
    for var in &manifest.exposes.env {
        validate_exposed_var(var, safety)
            .context("An exposed environment variable was misconfigured.")?;
    }

    Ok(())
}

fn validate_exposed_artifact(
    artifact: &ManifestExposesArtifact,
    artifact_kind: NyeFileEntryKind,
    directory: &NyeFileDirectory,
    safety: &Safety,
) -> anyhow::Result<()> {
    let path = artifact
        .path
        .to_str()
        .context("The exposed artifact's path could not be interpreted as a string.")?;

    if path.len() > safety.max_file_name_size as usize {
        anyhow::bail!("The exposed artifact's path was longer than allowed.");
    }
    if path.is_empty() {
        anyhow::bail!("The exposed artifact's path was empty.");
    }
    if artifact.link.len() > safety.max_link_size as usize {
        anyhow::bail!("The exposed artifact's link was longer than allowed.");
    }
    if artifact.link.is_empty() {
        anyhow::bail!("The exposed artifact's link was empty.");
    }

    nye_validation::is_safe_path(&artifact.path)
        .context("The exposed artifact's path was unsafe.")?;
    nye_validation::is_safe_path_component(&artifact.link)
        .context("The exposed artifact's link was unsafe or invalid.")?;

    let entry = directory.get_entry_by_path(
        artifact_kind,
        Segments::from_str(path).context(concat!(
            "The path specified by the exposed artifact could not be converted to its segments ",
            "representation."
        ))?,
    );

    if entry.is_none() {
        anyhow::bail!(
            "The exposed artifact's path did not point to an existing entry in the package file."
        );
    }

    Ok(())
}

fn validate_exposed_var(var: &ManifestExposesEnv, safety: &Safety) -> anyhow::Result<()> {
    if var.name.len() as u64 > safety.max_var_name_size {
        anyhow::bail!("The exposed env var's name is longer than allowed.");
    }
    if var.value.len() as u64 > safety.max_var_value_size {
        anyhow::bail!("The exposed env var's value is longer than allowed.");
    }

    if var.name.is_empty() {
        anyhow::bail!("The exposed variable's name was empty.");
    }

    nye_validation::is_env_var_name(&var.name)
        .context("The exposed environment variable's name was not valid.")?;

    Ok(())
}
