use std::collections::{HashMap, HashSet};

use anyhow::Context;
use nye_utils::permissions;
use nye_validation::Validate;
use serde::{Deserialize, Serialize};
use tokio::fs;
use url::Url;

use crate::registries::RegistryKind;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct RegistriesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub registries: HashMap<String, RegistryConfig>,
}

impl RegistriesConfig {
    pub fn get_config(&self, name: impl Into<String>) -> Option<&RegistryConfig> {
        let name = name.into();

        for (registry_name, registry_config) in &self.registries {
            if &name == registry_name
                || registry_config.aliases.contains(&name)
                || name == registry_config.source.to_string()
            {
                return Some(registry_config);
            }
        }

        None
    }

    pub fn get_default_config(&self) -> Option<&RegistryConfig> {
        if let Some(name) = &self.default {
            self.registries.get(name)
        } else {
            None
        }
    }

    pub fn name_collides(&self, name: impl Into<String>) -> bool {
        let name = name.into();

        for (registry_name, registry_config) in &self.registries {
            if &name == registry_name
                || registry_config.aliases.contains(&name)
                || name == registry_config.source.to_string()
            {
                return true;
            }
        }

        false
    }
}

impl Validate for RegistriesConfig {
    fn validate(&self) -> anyhow::Result<()> {
        if let Some(default) = &self.default
            && !self.registries.contains_key(default)
        {
            anyhow::bail!(concat!(
                "The default registry is not configured as a registry in the registries.toml ",
                "configuration file."
            ));
        }

        let mut names = HashSet::new();
        for (registry_name, registry_config) in &self.registries {
            validate_registry_name(registry_name.as_str())
                .context("One or more configured registry names are not valid.")?;

            registry_config
                .validate()
                .context("One or more registry configurations are invalid.")?;

            if names.contains(registry_name) {
                anyhow::bail!(
                    "The name `{registry_name}` is used for more than one registry. Names and aliases must be unique."
                );
            }
            for alias in &registry_config.aliases {
                if names.contains(alias) {
                    anyhow::bail!(
                        "The alias `{alias}` is used for more than one registry. Names and aliases must be unique."
                    );
                }
            }

            names.insert(registry_name.clone());
            names.extend(registry_config.aliases.clone());
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegistryConfig {
    pub source: Url,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub aliases: HashSet<String>,
    pub version: u8,
}

impl RegistryConfig {
    pub fn kind(&self) -> RegistryKind {
        match self.source.scheme() {
            "local" => RegistryKind::Local,
            "http" | "https" => RegistryKind::Http,
            _ => unreachable!(concat!(
                "config::RegistryConfig::kind() was called on an invalid config. Validate it ",
                "before calling. This is a bug."
            )),
        }
    }
}

impl Validate for RegistryConfig {
    fn validate(&self) -> anyhow::Result<()> {
        if self.version != 1 {
            anyhow::bail!(concat!(
                "The version used for the registry is not supported. Did you make a mistake? Is ",
                "nye up to date?"
            ));
        }

        match self.source.scheme() {
            "local" => {
                if self.source.has_authority() {
                    anyhow::bail!(
                        "Local registry URLs cannot have usernames, passwords, hostnames, nor ports set up."
                    );
                }

                let path = match self.source.to_file_path() {
                    Ok(v) => v,
                    Err(_) => anyhow::bail!(
                        "The directory path of the local URL could not be resolved to an absolute path."
                    ),
                };

                if !path.is_dir() {
                    anyhow::bail!(concat!(
                        "The path the local URL points to is not a directory in the system. Make ",
                        "sure it's pointing to one. Additionally, this error may get raised if ",
                        "there's a broken symlink in the path or there are missing permissions ",
                        "for you to read it."
                    ));
                }

                let permissions = permissions::get_current_user_permissions(path).context(
                    "Could not get current user permissions for local registry's directory.",
                )?;

                if !permissions.read {
                    anyhow::bail!(
                        "You don't have permission to read from the local registry's directory."
                    );
                }
                if !permissions.execute {
                    anyhow::bail!(
                        "You don't have permission to list entries in the local registry's directory."
                    );
                }
            }
            "http" | "https" => {
                if !self.source.has_host() {
                    anyhow::bail!("HTTP(S) registries need to have a hostname set up.");
                }

                if !self.source.username().is_empty() || self.source.password().is_some() {
                    anyhow::bail!(concat!(
                        "Sign into HTTP(S) registries using `nye signin` instead of setting your ",
                        "username and password in the registry config source URL."
                    ));
                }
            }
            _ => anyhow::bail!("Only `local`, `http`, and `https` registry schemes are supported."),
        }

        if !self.source.path().ends_with("/") {
            anyhow::bail!(concat!(
                "Registry source URLs must end with a slash. E.g. `https://pkg.nyeki.dev/v1/`, ",
                "not `https://pkg.nyeki.dev/v1`."
            ));
        }

        Ok(())
    }
}

fn validate_registry_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("Registries must have a non-empty name.");
    }

    if name.starts_with("-") {
        anyhow::bail!("Registry names cannot start with `-` not to conflict with CLI flags.");
    }

    for ch in name.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' && ch != '.' {
            anyhow::bail!(
                "Registry names must only contain lowercase ASCII letters, digits, `-`, and/or `.`."
            );
        }
    }

    Ok(())
}

/// Loads the configuration from `registries.toml`.
pub async fn load() -> anyhow::Result<RegistriesConfig> {
    let etc = nye_environment::get_env_etc()
        .context("Could not read etc directory path to load the registries configuration.")?;
    let path = etc.join("registries.toml");

    if fs::try_exists(&path)
        .await
        .context("Could not check if the registries.toml configuration existed.")?
    {
        let string = fs::read_to_string(&path)
            .await
            .context("Could not read registries.toml to string.")?;
        let config: RegistriesConfig =
            toml::from_str(&string).context("Could not parse TOML from registries.toml.")?;

        config
            .validate()
            .context("The registries.toml configuration file had an invalid configuration.")?;

        Ok(config)
    } else {
        Ok(RegistriesConfig::default())
    }
}

/// Writes the configuration into `registries.toml`.
pub async fn save(config: &RegistriesConfig) -> anyhow::Result<()> {
    let etc = nye_environment::get_env_etc()
        .context("Could not read etc directory path to save the registries configuration.")?;
    let path = etc.join("registries.toml");

    if !fs::try_exists(&path)
        .await
        .context("Could not check if registries.toml existed.")?
    {
        fs::create_dir_all(
            path.parent()
                .context("Could not get parent of registries.toml path")?,
        )
        .await
        .context("Could not create parent directories for registry.toml.")?;
    }

    let string = toml::to_string_pretty(config)
        .context("Could not serialize registries.toml config into toml.")?;

    fs::write(path, string)
        .await
        .context("Could not write updated registries.toml to registries.toml file.")?;

    Ok(())
}
