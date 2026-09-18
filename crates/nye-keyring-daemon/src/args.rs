/// Nye's registry credential manager daemon.
#[derive(clap::Parser)]
pub struct Args {
    /// Skip the root user check. Only in debug builds.
    #[cfg(debug_assertions)]
    #[arg(short, long)]
    pub no_root: bool,
}
