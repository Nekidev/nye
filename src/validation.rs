use std::collections::HashSet;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use anyhow::Context;
use path_clean::PathClean;
use regex::Regex;

use crate::targets::Target;

pub fn is_kebab_case(string: &str) -> anyhow::Result<()> {
    let pattern = Regex::new("^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap();

    if !pattern.is_match(string) {
        anyhow::bail!(concat!(
            "The string was not in kebab case. Only lowercase letters, numbers, and - are ",
            "supported, and it must not start with a -."
        ))
    } else {
        Ok(())
    }
}

pub fn is_safe_path(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();

    if path.is_absolute() {
        anyhow::bail!("Absolute paths are not safe.");
    }

    if path.clean() != path {
        anyhow::bail!(concat!(
            "Unclean paths are not safe. They must not be separated by double slashes (a//b) nor ",
            "contain any path traversal segments (..)."
        ));
    }

    for component in path.components() {
        is_safe_path_component(
            component
                .as_os_str()
                .to_str()
                .context("Path components must not contain unicode characters.")?,
        )
        .context("An individual segment of the path was not safe.")?;
    }

    Ok(())
}

pub fn is_safe_path_component(string: &str) -> anyhow::Result<()> {
    let path = PathBuf::from(string);

    if path.clean() != path {
        anyhow::bail!(concat!(
            "The provided path segment is not clean. It must not contain double slashes (//) nor ",
            "any path traversing (..)."
        ));
    }

    if string.contains('/') {
        anyhow::bail!(concat!(
            "The provided path segment contained a slash. Path segments are not full paths, ",
            "therefore must not contain slashes."
        ));
    }

    if string == ".." || string == "." {
        anyhow::bail!(concat!(
            "The provided path segment contained a reference to the current directory (./) or to ",
            "a parent directory (../). This is not safe."
        ));
    }

    for c in string.chars() {
        if !c.is_ascii_graphic() && c != ' ' {
            anyhow::bail!(concat!(
                "Only graphic ASCII characters and whitespaces are allowed in file paths ",
                "components."
            ))
        }
    }

    Ok(())
}

pub fn is_supported_target(target: &Target) -> anyhow::Result<()> {
    if !target.is_supported() {
        anyhow::bail!(concat!(
            "The provided target was properly formatted but not supported by nye. Only some ",
            "Linux targets are supported at the moment."
        ));
    }

    Ok(())
}

pub fn is_env_var_name(string: &str) -> anyhow::Result<()> {
    let pattern = Regex::new("^[a-zA-Z0-9_]+$").unwrap();

    if !pattern.is_match(string) {
        anyhow::bail!(concat!(
            "The provided environment variable name was invalid. They may only contain ASCII ",
            "letters, numbers, and _."
        ));
    }
    Ok(())
}

pub fn are_items_unique(items: &[impl Hash + Eq]) -> anyhow::Result<()> {
    let mut found = HashSet::new();

    for item in items {
        if found.contains(&item) {
            anyhow::bail!("An item provided was not unique. They must be.");
        }

        found.insert(item);
    }

    Ok(())
}

pub trait Validate {
    fn validate(&self) -> anyhow::Result<()>;
}
