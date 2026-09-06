use askama::Template;

use crate::semver::Semver;

#[derive(Template)]
#[template(path = "exposed-binary.jinja")]
pub struct BinaryWrapper {
    pub namespace: String,
    pub package: BinaryWrapperPackage,
    pub binary: BinaryWrapperBinary,
    pub declared_variables: Vec<BinaryWrapperDeclaredVariable>,
    pub consumed_variables: Vec<BinaryWrapperConsumedVariable>,
}

pub struct BinaryWrapperPackage {
    pub name: String,
    pub version: Semver,
}

pub struct BinaryWrapperBinary {
    /// The full path to the physical location of the binary.
    pub path: String,
}

pub struct BinaryWrapperDeclaredVariable {
    pub name: String,
    pub value: String,
}

pub struct BinaryWrapperConsumedVariable {
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
