use std::net::SocketAddr;
use std::path::PathBuf;

use clap::ArgAction;
use url::Url;

#[derive(clap::Args)]
pub struct Args {
    /// Display instructions on how to use nye registry.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,

    #[command(subcommand)]
    pub subcommand: Subcommand,
}

#[allow(clippy::large_enum_variant)]
#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Run an HTTP registry server.
    #[clap(visible_alias = "r")]
    Run(RunSubcommandArgs),

    /// Toasty development migration commands.
    #[cfg(debug_assertions)]
    #[command(visible_alias = "t")]
    Toasty(ToastySubcommandArgs),
}

#[derive(clap::Args)]
pub struct RunSubcommandArgs {
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
    pub registry: RunSubcommandArgsRegistry,

    #[clap(flatten)]
    pub duckity: Option<RunSubcommandArgsDuckity>,

    #[clap(flatten)]
    pub storage_s3: Option<RunSubcommandArgsStorageS3>,
    #[clap(flatten)]
    pub storage_file: Option<RunSubcommandArgsStorageLocal>,

    /// Display instructions on how to use nye registry run.
    #[arg(short, long, action = ArgAction::Help)]
    pub help: Option<bool>,
}

#[derive(clap::Args, Clone)]
pub struct RunSubcommandArgsRegistry {
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

#[derive(clap::Args, Clone)]
#[group(
    requires = "application_secret",
    requires = "signin_policy_id",
    requires = "signup_policy_id"
)]
pub struct RunSubcommandArgsDuckity {
    /// The Duckity application secret to use to protect sign in and sign up
    /// endpoints.
    #[arg(
        required = false,
        short = 'c',
        long = "duckity-application-secret",
        env = "NYE_REGISTRY_DUCKITY_APPLICATION_SECRET"
    )]
    pub application_secret: String,

    /// The Duckity policy ID to use in the sign in endpoint.
    #[arg(
        required = false,
        short = 'i',
        long = "duckity-singin-policy-id",
        env = "NYE_REGISTRY_DUCKITY_SIGNIN_POLICY_ID"
    )]
    pub signin_policy_id: String,

    /// The Duckity policy ID to use in the sign up endpoint.
    #[arg(
        required = false,
        short = 'u',
        long = "duckity-singup-policy-id",
        env = "NYE_REGISTRY_DUCKITY_SIGNUP_POLICY_ID"
    )]
    pub signup_policy_id: String,
}

#[derive(clap::Args, Clone)]
#[group(
    requires = "region",
    requires = "endpoint_url",
    requires = "bucket_name",
    requires = "access_key",
    requires = "secret_key"
)]
pub struct RunSubcommandArgsStorageS3 {
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

#[derive(clap::Args, Clone)]
#[group(requires = "location")]
pub struct RunSubcommandArgsStorageLocal {
    /// The path to the local directory under which to store package files.
    #[arg(
        required = false,
        short,
        long = "local-location",
        env = "NYE_REGISTRY_STORAGE_LOCAL_LOCATION"
    )]
    pub location: PathBuf,
}

#[cfg(debug_assertions)]
#[derive(clap::Args)]
pub struct ToastySubcommandArgs {
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
