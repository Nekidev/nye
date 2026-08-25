//! Semver 2.0.0 parsing, comparing, and serde.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::str::FromStr;

use anyhow::Context;
use regex::Regex;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};

/// A semantic version v2.0.0, as defined by https://semver.org/.
///
/// Parse from a string using [`FromStr`].
#[derive(Debug, PartialEq, Eq)]
pub struct Semver<T = u64>
where
    T: Number,
{
    pub major: T,
    pub minor: T,
    pub patch: T,
    pub prerelease: Vec<Part<T>>,
    pub build: Vec<Part<T>>,
}

impl<T> Semver<T>
where
    T: Number,
{
    /// Checks whether this version is compatible with another version.
    ///
    /// When two versions are compatible, no breaking changes occur between them.
    ///
    /// Checking if A is compatible with B is not the same as checking if B is compatible with A.
    /// For example,
    ///
    /// ```
    /// let a = Semver::from_str("1.0.0").unwrap();
    /// let b = Semver::from_str("1.1.0").unwrap();
    ///
    /// assert!(a.is_compatible(b));
    /// assert!(!b.is_compatible(a))
    /// ```
    ///
    /// That is because bigger minor versions on the same major version may introduce new features
    /// that the lower minor version does not have. However, changes on bigger minor versions will
    /// not be breaking (unless the major is 0), so software compatible with a smaller minor
    /// version will be compatible with greater minor versions.
    ///
    /// Rules:
    /// * If major versions differ, this is false.
    /// * If major versions are 0 and minor versions differ, this is false.
    /// * If this version's minor is greater than the other version's minor, this is false.
    /// * If prerelease versions differ, this is false.
    /// * If none of the checks above are false, this is true.
    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.major != other.major {
            return false;
        }

        if self.major == T::ZERO && self.minor != other.minor {
            return false;
        }

        if self.minor > other.minor {
            return false;
        }

        if self.prerelease != other.prerelease {
            return false;
        }

        true
    }
}

impl<T> FromStr for Semver<T>
where
    T: Number,
{
    type Err = anyhow::Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let mut prerelease = Vec::new();
        let mut build = Vec::new();

        if let Some((version, build_string)) = s.split_once("+") {
            build = parse_parts(build_string)
                .context("Could not parse build metadata from version.")?;
            s = version;
        }

        if let Some((version, prerelease_string)) = s.split_once("-") {
            prerelease = parse_parts(prerelease_string)
                .context("Could not parse prerelease from version.")?;
            s = version;
        }

        let parts: Vec<&str> = s.split(".").collect();
        if parts.len() != 3 {
            anyhow::bail!("Version did not have 3 numeric parts (X.Y.Z).");
        }

        let major = parse_version(parts[0]).context(
            "Could not parse major version from semver version because it was not a valid number.",
        )?;
        let minor = parse_version(parts[1]).context(
            "Could not parse major version from semver version because it was not a valid number.",
        )?;
        let patch = parse_version(parts[2]).context(
            "Could not parse major version from semver version because it was not a valid number.",
        )?;

        Ok(Semver {
            major,
            minor,
            patch,
            prerelease,
            build,
        })
    }
}

fn parse_version<T>(version: &str) -> anyhow::Result<T>
where
    T: Number,
{
    if version.starts_with("0") && version.len() > 1 {
        anyhow::bail!(
            "Major, minor, and patch versions cannot have leading 0s unless the number is 0."
        )
    }

    version
        .parse()
        .context("Could not parse major, minor, or patch version due to it not being numeric.")
}

fn parse_parts<T>(string: &str) -> anyhow::Result<Vec<Part<T>>>
where
    T: Number,
{
    let parts: Vec<&str> = string.split(".").collect();
    let mut result = Vec::new();

    for part in &parts {
        let part = Part::from_str(part).context("Could not parse part.")?;
        result.push(part);
    }

    Ok(result)
}

impl PartialOrd for Semver {
    fn gt(&self, other: &Self) -> bool {
        if self.major != other.major {
            return self.major > other.major;
        }

        if self.minor != other.minor {
            return self.minor > other.minor;
        }

        if self.patch != other.patch {
            return self.patch > other.patch;
        }

        if self.prerelease != other.prerelease {
            if self.prerelease.is_empty() {
                return true;
            }

            if other.prerelease.is_empty() {
                return false;
            }

            for i in 0..self.prerelease.len().max(other.prerelease.len()) {
                let a = self.prerelease.get(i);
                let b = other.prerelease.get(i);

                match (a, b) {
                    (Some(a), Some(b)) => {
                        if a != b {
                            return a > b;
                        }
                    }
                    (Some(_), None) => {
                        return true;
                    }
                    (None, Some(_)) => {
                        return false;
                    }
                    (None, None) => {
                        unreachable!(
                            "Both semver prerelease parts A and B were None in > check, this is a bug."
                        )
                    }
                }
            }
        }

        false
    }

    fn ge(&self, other: &Self) -> bool {
        if self.major != other.major {
            return self.major >= other.major;
        }

        if self.minor != other.minor {
            return self.minor >= other.minor;
        }

        if self.patch != other.patch {
            return self.patch >= other.patch;
        }

        if self.prerelease != other.prerelease {
            if self.prerelease.is_empty() {
                return true;
            }

            if other.prerelease.is_empty() {
                return false;
            }

            for i in 0..self.prerelease.len().max(other.prerelease.len()) {
                let a = self.prerelease.get(i);
                let b = other.prerelease.get(i);

                match (a, b) {
                    (Some(a), Some(b)) => {
                        if a != b {
                            return a >= b;
                        }
                    }
                    (Some(_), None) => {
                        return true;
                    }
                    (None, Some(_)) => {
                        return false;
                    }
                    (None, None) => {
                        unreachable!(
                            "Both semver prerelease parts A and B were None in >= check, this is a bug."
                        )
                    }
                }
            }
        }

        true
    }

    fn lt(&self, other: &Self) -> bool {
        if self.major != other.major {
            return self.major < other.major;
        }

        if self.minor != other.minor {
            return self.minor < other.minor;
        }

        if self.patch != other.patch {
            return self.patch < other.patch;
        }

        if self.prerelease != other.prerelease {
            if self.prerelease.is_empty() {
                return false;
            }

            if other.prerelease.is_empty() {
                return true;
            }

            for i in 0..self.prerelease.len().max(other.prerelease.len()) {
                let a = self.prerelease.get(i);
                let b = other.prerelease.get(i);

                match (a, b) {
                    (Some(a), Some(b)) => {
                        if a != b {
                            return a < b;
                        }
                    }
                    (Some(_), None) => {
                        return false;
                    }
                    (None, Some(_)) => {
                        return true;
                    }
                    (None, None) => {
                        unreachable!(
                            "Both semver prerelease parts A and B were None in < check, this is a bug."
                        )
                    }
                }
            }
        }

        false
    }

    fn le(&self, other: &Self) -> bool {
        if self.major != other.major {
            return self.major <= other.major;
        }

        if self.minor != other.minor {
            return self.minor <= other.minor;
        }

        if self.patch != other.patch {
            return self.patch <= other.patch;
        }

        if self.prerelease != other.prerelease {
            if self.prerelease.is_empty() {
                return false;
            }

            if other.prerelease.is_empty() {
                return true;
            }

            for i in 0..self.prerelease.len().max(other.prerelease.len()) {
                let a = self.prerelease.get(i);
                let b = other.prerelease.get(i);

                match (a, b) {
                    (Some(a), Some(b)) => {
                        if a != b {
                            return a <= b;
                        }
                    }
                    (Some(_), None) => {
                        return false;
                    }
                    (None, Some(_)) => {
                        return true;
                    }
                    (None, None) => {
                        unreachable!(
                            "Both semver prerelease parts A and B were None in <= check, this is a bug."
                        )
                    }
                }
            }
        }

        true
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.gt(other), self.lt(other)) {
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => Some(Ordering::Equal),
            (true, true) => unreachable!(
                "Incorrect PartialOrd implementation for utils::semver::Semver, both > and < returned true."
            ),
        }
    }
}

impl Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prerelease = self
            .prerelease
            .iter()
            .map(Part::to_string)
            .collect::<Vec<_>>()
            .join(".");
        let build = self
            .build
            .iter()
            .map(Part::to_string)
            .collect::<Vec<_>>()
            .join(".");

        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if !prerelease.is_empty() {
            write!(f, "-{prerelease}")?;
        }

        if !build.is_empty() {
            write!(f, "+{build}")?;
        }

        Ok(())
    }
}

impl Serialize for Semver {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Semver {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SemverVisitor;

        impl<'de> Visitor<'de> for SemverVisitor {
            type Value = Semver;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "A semver 2.0.0-compliant version string, without a v prefix."
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Semver::from_str(v).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(SemverVisitor)
    }
}

impl<T> From<(T, T, T)> for Semver<T>
where
    T: Number,
{
    fn from((major, minor, patch): (T, T, T)) -> Self {
        Semver {
            major,
            minor,
            patch,
            prerelease: Vec::new(),
            build: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part<T>
where
    T: Number,
{
    Number(T),
    String(String),
}

impl<T> FromStr for Part<T>
where
    T: Number,
{
    type Err = anyhow::Error;

    fn from_str(part: &str) -> Result<Self, Self::Err> {
        let pattern_string = Regex::new(r"^[a-zA-Z0-9-]+$").unwrap();
        let pattern_number = Regex::new(r"^(0|[1-9][0-9]*)$").unwrap();

        if !part.is_ascii() {
            anyhow::bail!("Part `{part}` is not ascii.");
        }

        if part.is_empty() {
            anyhow::bail!("Part is empty.");
        }

        let is_numeric = {
            let mut v = true;

            for c in part.chars() {
                if !c.is_ascii_digit() {
                    v = false;
                    break;
                }
            }

            v
        };

        let is_valid = match is_numeric {
            true => pattern_number.is_match(part),
            false => pattern_string.is_match(part),
        };

        if !is_valid {
            anyhow::bail!("Part `{part}` is not a valid semver part.");
        }

        if is_numeric {
            Ok(Part::Number(part.parse().unwrap()))
        } else {
            Ok(Part::String(part.to_string()))
        }
    }
}

impl<T> PartialOrd for Part<T>
where
    T: Number,
{
    fn gt(&self, other: &Self) -> bool {
        match (self, other) {
            (Part::Number(a), Part::Number(b)) => a > b,
            (Part::Number(_), Part::String(_)) => false,
            (Part::String(_), Part::Number(_)) => true,
            (Part::String(a), Part::String(b)) => a > b,
        }
    }

    fn ge(&self, other: &Self) -> bool {
        match (self, other) {
            (Part::Number(a), Part::Number(b)) => a >= b,
            (Part::Number(_), Part::String(_)) => false,
            (Part::String(_), Part::Number(_)) => true,
            (Part::String(a), Part::String(b)) => a >= b,
        }
    }

    fn lt(&self, other: &Self) -> bool {
        match (self, other) {
            (Part::Number(a), Part::Number(b)) => a < b,
            (Part::Number(_), Part::String(_)) => true,
            (Part::String(_), Part::Number(_)) => false,
            (Part::String(a), Part::String(b)) => a < b,
        }
    }

    fn le(&self, other: &Self) -> bool {
        match (self, other) {
            (Part::Number(a), Part::Number(b)) => a <= b,
            (Part::Number(_), Part::String(_)) => true,
            (Part::String(_), Part::Number(_)) => false,
            (Part::String(a), Part::String(b)) => a <= b,
        }
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self.gt(other), self.lt(other)) {
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => Some(Ordering::Equal),
            (true, true) => unreachable!(
                "Incorrect PartialOrd implementation for utils::semver::Part, both > and < returned true."
            ),
        }
    }
}

impl<T> Display for Part<T>
where
    T: Number,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Part::Number(v) => write!(f, "{v}"),
            Part::String(v) => write!(f, "{v}"),
        }
    }
}

/// A version numeric part type.
///
/// This trait is implemented by:
/// * [`u8`]
/// * [`u16`]
/// * [`u32`]
/// * [`u64`]
/// * [`u128`]
///
/// Either of those types can be passed as a type parameter to [`Semver`]. [`Semver::major`],
/// minor, patch, and prerelease and build parts will parse numeric values as the specified type.
pub trait Number: Display + FromStr<Err: Debug + Send + Sync + Error + 'static> + Ord + Eq {
    const ZERO: Self;
}

impl Number for u8 {
    const ZERO: Self = 0;
}
impl Number for u16 {
    const ZERO: Self = 0;
}
impl Number for u32 {
    const ZERO: Self = 0;
}
impl Number for u64 {
    const ZERO: Self = 0;
}
impl Number for u128 {
    const ZERO: Self = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_valid() {
        let versions = [
            "0.0.4",
            "1.2.3",
            "10.20.30",
            "1.1.2-prerelease+meta",
            "1.1.2+meta",
            "1.1.2+meta-valid",
            "1.0.0-alpha",
            "1.0.0-beta",
            "1.0.0-alpha.beta",
            "1.0.0-alpha.beta.1",
            "1.0.0-alpha.1",
            "1.0.0-alpha0.valid",
            "1.0.0-alpha.0valid",
            "1.0.0-alpha-a.b-c-somethinglong+build.1-aef.1-its-okay",
            "1.0.0-rc.1+build.1",
            "2.0.0-rc.1+build.123",
            "1.2.3-beta",
            "10.2.3-DEV-SNAPSHOT",
            "1.2.3-SNAPSHOT-123",
            "1.0.0",
            "2.0.0",
            "1.1.7",
            "2.0.0+build.1848",
            "2.0.1-alpha.1227",
            "1.0.0-alpha+beta",
            "1.2.3----RC-SNAPSHOT.12.9.1--.12+788",
            "1.2.3----R-S.12.9.1--.12+meta",
            "1.2.3----RC-SNAPSHOT.12.9.1--.12",
            "1.0.0+0.build.1-rc.10000aaa-kk-0.1",
            "1.0.0-0A.is.legal",
        ];

        for version in &versions {
            let result = Semver::<u64>::from_str(version);
            assert!(
                result.is_ok(),
                "Semver::from_str(\"{version}\") == {result:?}"
            );
        }
    }

    #[test]
    fn test_semver_invalid() {
        let versions = [
            "1",
            "1.2",
            "1.2.3-0123",
            "1.2.3-0123.0123",
            "1.1.2+.123",
            "+invalid",
            "-invalid",
            "-invalid+invalid",
            "-invalid.01",
            "alpha",
            "alpha.beta",
            "alpha.beta.1",
            "alpha.1",
            "alpha+beta",
            "alpha_beta",
            "alpha.",
            "alpha..",
            "beta",
            "1.0.0-alpha_beta",
            "-alpha.",
            "1.0.0-alpha..",
            "1.0.0-alpha..1",
            "1.0.0-alpha...1",
            "1.0.0-alpha....1",
            "1.0.0-alpha.....1",
            "1.0.0-alpha......1",
            "1.0.0-alpha.......1",
            "01.1.1",
            "1.01.1",
            "1.1.01",
            "1.2",
            "1.2.3.DEV",
            "1.2-SNAPSHOT",
            "1.2.31.2.3----RC-SNAPSHOT.12.09.1--..12+788",
            "1.2-RC-SNAPSHOT",
            "-1.0.3-gamma+b7718",
            "+justmeta",
            "9.8.7+meta+meta",
            "9.8.7-whatever+meta+meta",
            "99999999999999999999999.999999999999999999.99999999999999999----RC-SNAPSHOT.12.09.1--------------------------------..12",
        ];

        for version in &versions {
            let result = Semver::<u128>::from_str(version);
            assert!(
                result.is_err(),
                "Semver::from_str(\"{version}\") == {result:?}"
            );
        }
    }

    #[test]
    fn test_semver_precedence() {
        // Left is always smaller than right.
        let tuples = [
            ("0.9.99", "1.0.0"),
            ("0.9.0", "0.10.0"),
            ("1.0.0-0.0", "1.0.0-0.0.0"),
            ("1.0.0-9999", "1.0.0--"),
            ("1.0.0-99", "1.0.0-100"),
            ("1.0.0-alpha", "1.0.0-alpha.1"),
            ("1.0.0-alpha.1", "1.0.0-alpha.beta"),
            ("1.0.0-alpha.beta", "1.0.0-beta"),
            ("1.0.0-beta", "1.0.0-beta.2"),
            ("1.0.0-beta.2", "1.0.0-beta.11"),
            ("1.0.0-beta.11", "1.0.0-rc.1"),
            ("1.0.0-rc.1", "1.0.0"),
            ("1.0.0-0", "1.0.0--1"),
            ("1.0.0-0", "1.0.0-1"),
            ("1.0.0-1.0", "1.0.0-1.-1"),
        ];

        for (a, b) in &tuples {
            let a_semver = Semver::from_str(a).unwrap();
            let b_semver = Semver::from_str(b).unwrap();

            assert!(a_semver < b_semver, "{a} < {b} was not true");
            assert!(b_semver > a_semver, "{b} > {a} was not true");
            assert!(a_semver <= b_semver, "{a} <= {b} was not true");
            assert!(b_semver >= a_semver, "{a} >= {b} was not true");
        }
    }

    #[test]
    fn test_semver_compatibility() {
        let tuples = [
            ("0.1.0", "0.1.1"),
            ("1.0.0", "1.0.0"),
            ("1.0.1", "1.1.0"),
            ("1.0.0+abc", "1.0.0+def"),
            ("1.0.0-alpha.1", "1.0.0-alpha.1"),
        ];

        for (a, b) in &tuples {
            let a_semver: Semver<u8> = Semver::from_str(a).unwrap();
            let b_semver: Semver<u8> = Semver::from_str(b).unwrap();

            assert!(
                a_semver.is_compatible(&b_semver),
                "{a} is not compatible with {b}"
            );
        }
    }

    #[test]
    fn test_semver_incompatibility() {
        let tuples = [
            ("1.0.0", "0.1.0"),
            ("0.1.1", "1.0.0"),
            ("0.1.0", "0.2.0"),
            ("1.0.0-rc.1", "1.0.0"),
            ("1.0.0-rc.1", "1.0.0-rc.2"),
        ];

        for (a, b) in &tuples {
            let a_semver: Semver<u8> = Semver::from_str(a).unwrap();
            let b_semver: Semver<u8> = Semver::from_str(b).unwrap();

            assert!(
                !a_semver.is_compatible(&b_semver),
                "{a} is compatible with {b}"
            );
        }
    }

    #[test]
    fn test_semver_display() {
        let versions = [
            "1.0.0",
            "1.0.0-alpha",
            "1.0.0-alpha.2",
            "1.0.0+build",
            "1.0.0+build.meta",
            "1.0.0+build.meta",
            "1.0.0-alpha+build.meta",
            "1.0.0-alpha.2+build.meta",
        ];

        for version in versions {
            let semver = Semver::from_str(version).unwrap();

            assert_eq!(version, semver.to_string());
        }
    }
}
