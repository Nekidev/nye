//! Registry configuration via `registries.toml`.
//!
//! Nye stores the registry configuration in `registries.toml` under its `etc` directory
//! (`/pkg/store/nye/0.0.0/etc/registries.toml` or similar). The file is simple, it looks something
//! like this:
//! 
//! ```toml
//! default = "pkg"                   # Optional, defaults to the first one defined.
//! 
//! [[registries]]
//! name = "pkg"                      # The name used to reference the registry, e.g. `nye i pkg/busybox`.
//! kind = "http"                     # Either `http` or `local`, more kinds may be added in the future.
//! url = "https://pkg.nyeki.dev/v1"  # The URL to the registry.
//! 
//! [[registries]]
//! name = "local"
//! kind = "local"
//! url = "local:/usr/nyeki/pkg/local"
//! ```
//! 
//! You can load the `registries.toml` file using [`RegistriesConfig::load()`]

use serde::{Deserialize, Serialize};
use url::Url;

use crate::RegistryKind;

/// `registries.toml`'s root schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistriesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistryConfig>,
}

/// A registry's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// The registry's name.
    pub name: String,
    /// The registry's URL.
    pub url: Url,
    /// The kind of registry.
    pub kind: RegistryKind,
}
