use std::env::consts::{ARCH, OS};
use std::fmt::Display;
use std::str::FromStr;

use anyhow::Context;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};

/// Rust's supported operating systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Linux,
    Windows,
    MacOS,
    Android,
    Ios,
    OpenBSD,
    FreeBSD,
    NetBSD,
    Wasi,
    Hermit,
    Aix,
    Apple,
    Dragonfly,
    Emscripten,
    Espidf,
    Fortanix,
    Uefi,
    Fuchsia,
    Haiku,
    WatchOS,
    VisionOS,
    TvOS,
    Horizon,
    Hurd,
    Illumos,
    L4re,
    Nto,
    Redox,
    Solaris,
    SolidASP3,
    Vexos,
    Vita,
    Vxworks,
    Xous,
}

impl FromStr for Os {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux" => Ok(Os::Linux),
            "windows" => Ok(Os::Windows),
            "macos" => Ok(Os::MacOS),
            "android" => Ok(Os::Android),
            "ios" => Ok(Os::Ios),
            "openbsd" => Ok(Os::OpenBSD),
            "freebsd" => Ok(Os::FreeBSD),
            "netbsd" => Ok(Os::NetBSD),
            "wasi" => Ok(Os::Wasi),
            "hermit" => Ok(Os::Hermit),
            "aix" => Ok(Os::Aix),
            "apple" => Ok(Os::Apple),
            "dragonfly" => Ok(Os::Dragonfly),
            "emscripten" => Ok(Os::Emscripten),
            "espidf" => Ok(Os::Espidf),
            "fortanix" => Ok(Os::Fortanix),
            "uefi" => Ok(Os::Uefi),
            "fuchsia" => Ok(Os::Fuchsia),
            "haiku" => Ok(Os::Haiku),
            "watchos" => Ok(Os::WatchOS),
            "visionos" => Ok(Os::VisionOS),
            "tvos" => Ok(Os::TvOS),
            "horizon" => Ok(Os::Horizon),
            "hurd" => Ok(Os::Hurd),
            "illumos" => Ok(Os::Illumos),
            "l4re" => Ok(Os::L4re),
            "nto" => Ok(Os::Nto),
            "redox" => Ok(Os::Redox),
            "solaris" => Ok(Os::Solaris),
            "solid_asp3" => Ok(Os::SolidASP3),
            "vexos" => Ok(Os::Vexos),
            "vita" => Ok(Os::Vita),
            "vxworks" => Ok(Os::Vxworks),
            "xous" => Ok(Os::Xous),
            _ => anyhow::bail!("`{s}` is not a supported operating system."),
        }
    }
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// Rust's supported CPU architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    X86,
    X86_64,
    Arm,
    Aarch64,
    M68k,
    Mips,
    Mips32r6,
    Mips64,
    Mips64r6,
    Csky,
    Powerpc,
    Powerpc64,
    Riscv32,
    Riscv64,
    S390x,
    Sparc,
    Sparc64,
    Hexagon,
    Loongarch32,
    Loongarch64,
}

impl FromStr for Arch {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86" => Ok(Arch::X86),
            "x86_64" => Ok(Arch::X86_64),
            "arm" => Ok(Arch::Arm),
            "aarch64" => Ok(Arch::Aarch64),
            "m68k" => Ok(Arch::M68k),
            "mips" => Ok(Arch::Mips),
            "mips32r6" => Ok(Arch::Mips32r6),
            "mips64" => Ok(Arch::Mips64),
            "mips64r6" => Ok(Arch::Mips64r6),
            "csky" => Ok(Arch::Csky),
            "powerpc" => Ok(Arch::Powerpc),
            "powerpc64" => Ok(Arch::Powerpc64),
            "riscv32" => Ok(Arch::Riscv32),
            "riscv64" => Ok(Arch::Riscv64),
            "s390x" => Ok(Arch::S390x),
            "sparc" => Ok(Arch::Sparc),
            "sparc64" => Ok(Arch::Sparc64),
            "hexagon" => Ok(Arch::Hexagon),
            "loongarch32" => Ok(Arch::Loongarch32),
            "loongarch64" => Ok(Arch::Loongarch64),
            _ => anyhow::bail!("{s} is not a supported CPU architecture."),
        }
    }
}

impl Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// A system target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target(pub Os, pub Arch);

impl Target {
    /// Returns the current system's target, if a valid one.
    pub fn get_current() -> anyhow::Result<Target> {
        Ok(Target(
            Os::from_str(OS).context("Could not get current system's OS.")?,
            Arch::from_str(ARCH).context("Could not get curent system's CPU architecture.")?,
        ))
    }

    /// Returns whether nye supports this target.
    pub fn is_supported(&self) -> bool {
        self.0 == Os::Linux
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.0, self.1)
    }
}

impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((os, arch)) = s.split_once("-") else {
            anyhow::bail!("The target did not specify an architecture.")
        };

        let os = Os::from_str(os).context("The operating system of the target was invalid.")?;
        let arch =
            Arch::from_str(arch).context("The CPU architecture of the target was invalid.")?;

        Ok(Target(os, arch))
    }
}

impl Serialize for Target {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TargetVisitor;

        impl<'de> Visitor<'de> for TargetVisitor {
            type Value = Target;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A valid system target, e.g. linux-x86_64.")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Target::from_str(v).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(TargetVisitor)
    }
}
