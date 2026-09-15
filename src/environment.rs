use std::env::VarError;
use std::path::PathBuf;

use anyhow::Context;

fn get_env_xdg_config_home() -> anyhow::Result<Option<PathBuf>> {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(v) => {
            let path = PathBuf::from(v);

            if path.is_relative() {
                anyhow::bail!(concat!(
                    "The XDG_CONFIG_HOME env var contained a relative path. Only absolute paths ",
                    "are valid XDG env var values."
                ));
            }

            Ok(Some(path))
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => {
            anyhow::bail!("The XDG_CONFIG_HOME env var contained non-unicode bytes.")
        }
    }
}

fn get_env_nye_installation() -> anyhow::Result<Option<PathBuf>> {
    match std::env::var("NYE_INSTALLATION") {
        Ok(v) => Ok(Some(PathBuf::from(v))),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => {
            anyhow::bail!("The NYE_INSTALLATION env var contained non-unicode bytes.")
        }
    }
}

/// Returns the etc directory for this nye installation.
///
/// The value is read from the environment variables, using fallbacks in the following order:
/// 1. `$NYE_INSTALLATION/etc`
/// 2. `$XDG_CONFIG_HOME/nye`
/// 3. `$HOME/.config/nye`
///
/// Yes, we hate XDG and storing files in `$CONFIG`. The fallbacks are there because in a proper
/// nye installation `$NYE_INSTALLATION` will always be present, and if it's not then it's likely
/// a development environment without a nye-wrapped nye binary.
pub fn get_env_etc() -> anyhow::Result<PathBuf> {
    match get_env_nye_installation() {
        Ok(Some(path)) => return Ok(path.join("etc")),
        Ok(None) => {}
        Err(error) => return Err(error.context("Could not read the NYE_INSTALLATION env var.")),
    }

    match get_env_xdg_config_home() {
        Ok(Some(path)) => return Ok(path.join("nye")),
        Ok(None) => {}
        Err(error) => return Err(error.context("Could not read the XDG_CONFIG_HOME env var.")),
    }

    Ok(std::env::home_dir()
        .context("There is no home directory configured.")?
        .join(".config")
        .join("nye"))
}
