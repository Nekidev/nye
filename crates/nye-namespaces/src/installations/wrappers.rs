//! Wrappers for exposed binaries.

use askama::Template;
use nye_schemas::semver::Semver;

#[derive(Template)]
#[template(path = "wrapper.sh.j2")]
pub struct Wrapper {
    /// The namespace's root path.
    pub namespace: String,
    /// Package name and version, for display in comments.
    pub package: WrapperPackage,
    /// The path to the exposed binary.
    pub binary: WrapperBinary,
    /// Variables declared with a single fixed value.
    pub declared_variables: Vec<WrapperDeclaredVariable>,
    /// Variables consumed from exposed vars by other packages.
    pub consumed_variables: Vec<WrapperConsumedVariable>,
}

impl Wrapper {
    pub fn build(
        namespace: impl Into<String>,
        package_name: impl Into<String>,
        package_version: impl Into<Semver>,
        binary_path: impl Into<String>,
    ) -> WrapperBuilder {
        WrapperBuilder {
            namespace: namespace.into(),
            package: WrapperPackage {
                name: package_name.into(),
                version: package_version.into(),
            },
            binary: WrapperBinary {
                path: binary_path.into(),
            },
            declared_variables: Vec::new(),
            consumed_variables: Vec::new(),
        }
    }
}

pub struct WrapperPackage {
    pub name: String,
    pub version: Semver,
}

pub struct WrapperBinary {
    /// The full path to the physical location of the binary.
    pub path: String,
}

#[derive(Clone)]
pub struct WrapperDeclaredVariable {
    pub name: String,
    pub value: String,
}

#[derive(Clone)]
pub struct WrapperConsumedVariable {
    pub name: String,
    pub separator: String,
}

mod filters {
    use std::fmt::Display;

    #[askama::filter_fn]
    pub fn shell_quote(value: impl Display, _env: &dyn askama::Values) -> askama::Result<String> {
        let mut value = value.to_string();
        value = value.replace("\"", "\\\"");

        if value.ends_with('\\') {
            value.push('\\');
        }

        Ok(format!("\"{value}\""))
    }
}

pub struct WrapperBuilder {
    pub namespace: String,
    pub package: WrapperPackage,
    pub binary: WrapperBinary,
    pub declared_variables: Vec<WrapperDeclaredVariable>,
    pub consumed_variables: Vec<WrapperConsumedVariable>,
}

impl WrapperBuilder {
    pub fn new(
        namespace: impl Into<String>,
        package_name: impl Into<String>,
        package_version: impl Into<Semver>,
        binary_path: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            package: WrapperPackage {
                name: package_name.into(),
                version: package_version.into(),
            },
            binary: WrapperBinary {
                path: binary_path.into(),
            },
            declared_variables: Vec::new(),
            consumed_variables: Vec::new(),
        }
    }

    pub fn add_declared_variable(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> &mut Self {
        self.declared_variables.push(WrapperDeclaredVariable {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    pub fn with_declared_variable(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.declared_variables.push(WrapperDeclaredVariable {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    pub fn add_consumed_variable(
        &mut self,
        name: impl Into<String>,
        separator: impl Into<String>,
    ) -> &mut Self {
        self.consumed_variables.push(WrapperConsumedVariable {
            name: name.into(),
            separator: separator.into(),
        });
        self
    }

    pub fn with_consumed_variable(
        mut self,
        name: impl Into<String>,
        separator: impl Into<String>,
    ) -> Self {
        self.consumed_variables.push(WrapperConsumedVariable {
            name: name.into(),
            separator: separator.into(),
        });
        self
    }

    pub fn build(self) -> Wrapper {
        Wrapper {
            namespace: self.namespace,
            package: self.package,
            binary: self.binary,
            declared_variables: self.declared_variables,
            consumed_variables: self.consumed_variables,
        }
    }
}
