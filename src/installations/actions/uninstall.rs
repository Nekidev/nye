use anyhow::Context as AnyhowContext;
use tokio::fs;

use crate::installations::context::Context;
use crate::installations::database::{self, Package};

pub async fn uninstall(ctx: &Context, name: &str) -> anyhow::Result<()> {
    let mut db = database::connect(ctx.get_database_url())
        .await
        .context("Could not connect to state database.")?;
    let mut db = db
        .transaction()
        .await
        .context("Could not start a database transaction.")?;

    let package = Package::get_by_name(&mut db, name)
        .await
        .context(format!("Could not get package named {name}."))?;

    let bins = package
        .exposes_bins()
        .exec(&mut db)
        .await
        .context("Could not get exposed binaries of package.")?;
    let libs = package
        .exposes_libs()
        .exec(&mut db)
        .await
        .context("Could not get exposed libraries of package.")?;

    for bin in bins {
        let path = ctx.root.join("bin").join(bin.name);

        if fs::try_exists(&path)
            .await
            .context("Could not check if symlink to exposed binary existed.")?
        {
            fs::remove_file(&path)
                .await
                .context(format!("Could not delete symlink at `{}`.", path.display()))?;
        }
    }

    for lib in libs {
        let path = ctx.root.join("lib").join(lib.name);

        if fs::try_exists(&path)
            .await
            .context("Could not check if symlink to exposed library existed.")?
        {
            fs::remove_file(&path)
                .await
                .context(format!("Could not delete symlink at `{}`.", path.display()))?;
        }
    }

    let package_path = ctx.root.join("pkg").join("store").join(&package.name);
    let version_path = package_path.join(&package.version);

    fs::remove_dir_all(version_path)
        .await
        .context("Could not delete installed package version path.")?;

    let mut remaining_versions = fs::read_dir(&package_path)
        .await
        .context("Could not list remaining installed versions for package.")?;

    if remaining_versions
        .next_entry()
        .await
        .context("Could not get next entry in package's installed versions directory.")?
        .is_none()
    {
        fs::remove_dir(&package_path)
            .await
            .context("Could not delete empty package versions directory.")?;
    }

    package
        .delete()
        .exec(&mut db)
        .await
        .context("Could not remove uninstalled package from state database.")?;

    db.commit()
        .await
        .context("Could not commit state database transaction.")?;

    Ok(())
}
