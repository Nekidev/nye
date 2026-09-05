use nye::installations::database::{ExposedBin, Package};
use toasty_cli::{Config, ToastyCli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;

    let db = toasty::Db::builder()
        .models(toasty::models!(Package, ExposedBin))
        .connect("sqlite:./state.db")
        .await?;

    let cli = ToastyCli::with_config(db, config);
    cli.parse_and_run().await?;

    Ok(())
}
