#[cfg(feature = "registry")]
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::ArgAction;
#[cfg(any(debug_assertions, feature = "registry"))]
use url::Url;

use crate::targets::Target;

/// Nyeki's package manager.
#[derive(clap::Parser)]
#[clap(version)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub struct Args {
    #[arg(short, long, help = "Display the current system target.")]
    pub target: bool,

    #[arg(
        short,
        long,
        help = "Run the command using the system's installation context."
    )]
    pub system: bool,

    #[arg(short, long, help = "Display instructions on how to use nye.", action = ArgAction::Help)]
    pub help: Option<bool>,

    #[arg(short, long, help = "Display the current nye version.", action = ArgAction::Version)]
    pub version: Option<bool>,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[allow(clippy::large_enum_variant)]
#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Create, pack, and publish packages.
    #[command(visible_alias = "d")]
    Dev(DevSubcommandArgs),

    /// Install one or more packages.
    #[command(visible_alias = "i")]
    Install(InstallSubcommandArgs),

    /// Uninstall one or more packages.
    #[command(visible_alias = "u")]
    Uninstall(UninstallSubcommandArgs),

    /// Lists all installed packages.
    #[command(visible_alias = "l")]
    List(ListSubcommandArgs),

    /// Registry HTTP server commands.
    #[cfg(feature = "registry")]
    #[command(visible_alias = "r")]
    Registry(RegistrySubcommandArgs),

    /// Toasty development migration commands.
    #[cfg(debug_assertions)]
    #[command(visible_alias = "t")]
    Toasty(ToastySubcommandArgs),
}

#[derive(clap::Args)]
pub struct DevSubcommandArgs {
    #[command(subcommand)]
    pub subcommand: DevSubcommandSubcommand,

    /// Display instructions on how to use nye dev and its subcommands.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Subcommand)]
pub enum DevSubcommandSubcommand {
    /// Initialize a new package project.
    #[command(visible_alias = "i")]
    Init(DevSubcommandInitSubcommandArgs),

    /// Package the current project into an installable file.
    #[command(visible_alias = "p")]
    Pack(DevSubcommandPackSubcommandArgs),
}

#[derive(clap::Args)]
pub struct DevSubcommandInitSubcommandArgs {
    /// The directory to use for the new package project.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// The name to give the package project. Defaults to the path's directory
    /// name.
    #[arg(short, long)]
    pub name: Option<String>,

    /// Display instructions on how to use nye dev init.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct DevSubcommandPackSubcommandArgs {
    /// Filter the supported targets to package.
    #[arg(short, long = "target")]
    pub targets: Vec<Target>,

    /// Overwrite existing packages in the dist folder.
    #[arg(short, long)]
    pub overwrite: bool,

    /// Display instructions on how to use nye dev pack.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct InstallSubcommandArgs {
    /// The path to one or more installable package files.
    #[arg(short, long)]
    pub path: Vec<PathBuf>,

    /// Display instructions on how to use nye install.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct UninstallSubcommandArgs {
    /// The names of the packages to uninstall.
    pub packages: Vec<String>,

    /// Display instructions on how to use nye uninstall.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct ListSubcommandArgs {
    #[command(subcommand)]
    pub subcommand: Option<ListSubcommandSubcommand>,

    /// Display instructions on how to use nye list and its subcommands.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Subcommand)]
pub enum ListSubcommandSubcommand {
    /// Lists all exposed binaries by installed packages.
    #[clap(visible_alias = "b")]
    Bins(ListSubcommandBinsSubcommandArgs),

    /// Lists all exposed libraries by installed packages.
    #[clap(visible_alias = "l")]
    Libs(ListSubcommandLibsSubcommandArgs),
}

#[derive(clap::Args)]
pub struct ListSubcommandBinsSubcommandArgs {
    /// Display instructions on how to use nye list bins.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args)]
pub struct ListSubcommandLibsSubcommandArgs {
    /// Display instructions on how to use nye list libs.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[cfg(feature = "registry")]
#[derive(clap::Args)]
pub struct RegistrySubcommandArgs {
    /// Display instructions on how to use nye registry.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[command(subcommand)]
    pub subcommand: RegistrySubcommandSubcommand,
}

#[cfg(feature = "registry")]
#[allow(clippy::large_enum_variant)]
#[derive(clap::Subcommand)]
pub enum RegistrySubcommandSubcommand {
    /// Run an HTTP registry server.
    #[clap(visible_alias = "r")]
    Run(RegistrySubcommandRunSubcommandArgs),

    /// Toasty development migration commands.
    #[cfg(debug_assertions)]
    #[command(visible_alias = "t")]
    Toasty(RegistrySubcommandToastySubcommandArgs),
}

#[cfg(feature = "registry")]
#[derive(clap::Args)]
pub struct RegistrySubcommandRunSubcommandArgs {
    /// The address to listen for incoming connections at.
    #[arg(env = "NYE_REGISTRY_BIND", default_value = "127.0.0.1:3000")]
    pub bind: SocketAddr,

    /// The PostgreSQL database URL to run on.
    #[arg(
        short,
        long,
        env = "NYE_REGISTRY_DATABASE_URL",
        default_value = "postgres://postgres:postgres@localhost:5432/postgres"
    )]
    pub database_url: Url,

    #[clap(flatten)]
    pub registry: RegistrySubcommandRunSubcommandArgsRegistry,

    #[clap(flatten)]
    pub duckity: Option<RegistrySubcommandRunSubcommandArgsDuckity>,

    #[clap(flatten)]
    pub storage_s3: Option<RegistrySubcommandRunSubcommandArgsStorageS3>,
    #[clap(flatten)]
    pub storage_file: Option<RegistrySubcommandRunSubcommandArgsStorageLocal>,

    /// Display instructions on how to use nye registry run.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[cfg(feature = "registry")]
#[derive(clap::Args, Clone)]
pub struct RegistrySubcommandRunSubcommandArgsRegistry {
    /// The registry name users will see when setting up your registry.
    #[arg(
        short,
        long = "registry-name",
        env = "NYE_REGISTRY_NAME",
        default_value = "A Nye Package Registry"
    )]
    pub name: String,

    /// Disable account creation.
    #[arg(
        short = 'f',
        long = "registry-no-signup",
        env = "NYE_REGISTRY_NO_SIGNUP",
        action = ArgAction::SetTrue
    )]
    pub no_signup: bool,
    /// Disable account login.
    #[arg(
        short = 'g',
        long = "registry-no-signin",
        env = "NYE_REGISTRY_NO_SIGNIN",
        action = ArgAction::SetTrue
    )]
    pub no_signin: bool,
}

#[cfg(feature = "registry")]
#[derive(clap::Args, Clone)]
#[group(
    requires = "application_secret",
    requires = "signin_protection_profile_id",
    requires = "signup_protection_profile_id"
)]
pub struct RegistrySubcommandRunSubcommandArgsDuckity {
    /// The Duckity application secret to use to protect sign in and sign up
    /// endpoints.
    #[arg(
        required = false,
        short = 'c',
        long = "duckity-application-secret",
        env = "NYE_REGISTRY_DUCKITY_APPLICATION_SECRET"
    )]
    pub application_secret: String,

    /// The Duckity protection profile ID to use in the sign in endpoint.
    #[arg(
        required = false,
        short = 'i',
        long = "duckity-singin-protection-profile-id",
        env = "NYE_REGISTRY_DUCKITY_SIGNIN_PROTECTION_PROFILE_ID"
    )]
    pub signin_protection_profile_id: String,

    /// The Duckity protection profile ID to use in the sign up endpoint.
    #[arg(
        required = false,
        short = 'u',
        long = "duckity-singup-protection-profile-id",
        env = "NYE_REGISTRY_DUCKITY_SIGNUP_PROTECTION_PROFILE_ID"
    )]
    pub signup_protection_profile_id: String,
}

#[cfg(feature = "registry")]
#[derive(clap::Args, Clone)]
#[group(
    requires = "region",
    requires = "endpoint_url",
    requires = "bucket_name",
    requires = "access_key",
    requires = "secret_key"
)]
pub struct RegistrySubcommandRunSubcommandArgsStorageS3 {
    /// The S3 region where to store package files.
    #[arg(
        required = false,
        short = 'r',
        long = "s3-region",
        env = "NYE_REGISTRY_STORAGE_S3_REGION"
    )]
    pub region: String,

    /// The S3 endpoint URL where to store package files.
    #[arg(
        required = false,
        short = 'e',
        long = "s3-endpoint-url",
        env = "NYE_REGISTRY_STORAGE_S3_ENDPOINT_URL"
    )]
    pub endpoint_url: String,

    /// The name of the S3 bucket where to store package files.
    #[arg(
        required = false,
        short = 'b',
        long = "s3-bucket-name",
        env = "NYE_REGISTRY_STORAGE_S3_BUCKET_NAME"
    )]
    pub bucket_name: String,

    /// The access key to the S3 bucket where to store package files.
    #[arg(
        required = false,
        short = 'a',
        long = "s3-access-key",
        env = "NYE_REGISTRY_STORAGE_S3_ACCESS_KEY"
    )]
    pub access_key: String,

    /// The secret key to the S3 bucket where to store package files.
    #[arg(
        required = false,
        short = 's',
        long = "s3-secret-key",
        env = "NYE_REGISTRY_STORAGE_S3_SECRET_KEY"
    )]
    pub secret_key: String,
}

#[cfg(feature = "registry")]
#[derive(clap::Args, Clone)]
#[group(requires = "location")]
pub struct RegistrySubcommandRunSubcommandArgsStorageLocal {
    /// The path to the local directory under which to store package files.
    #[arg(
        required = false,
        short,
        long = "local-location",
        env = "NYE_REGISTRY_STORAGE_LOCAL_LOCATION"
    )]
    pub location: PathBuf,
}

#[cfg(all(debug_assertions, feature = "registry"))]
#[derive(clap::Args)]
pub struct RegistrySubcommandToastySubcommandArgs {
    /// The arguments to pass to the toasty command.
    pub args: Vec<String>,

    /// The PostgreSQL database URL to use to generate migrations.
    #[arg(
        short,
        long,
        default_value = "postgres://postgres:postgres@localhost:5432/postgres"
    )]
    pub database_url: Url,

    /// Display instructions on how to use nye registry toasty.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[cfg(debug_assertions)]
#[derive(clap::Args)]
pub struct ToastySubcommandArgs {
    /// The arguments to pass to the toasty command.
    pub args: Vec<String>,

    /// The SQLite database URL to use to generate migrations.
    #[arg(short, long, default_value = "sqlite://state.db")]
    pub database_url: Url,

    /// Display instructions on how to use nye toasty.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}
